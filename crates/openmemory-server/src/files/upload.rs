use bytes::Bytes;
use openmemory_ops::{PagePath, scan_secrets};

use crate::access::Actor;
use crate::files::mime::{AllowedMime, FileKind};
use crate::files::queries::{self, NewFile};
use crate::files::{FileContent, FileRecord, Files, FilesError, pdf};
use crate::storage::{self, BlobKey, Sha256Hash};

const PATH_LIVE_KEY: &str = "files_path_live_key";

impl Files {
    pub(crate) async fn upload(
        &self,
        actor: &Actor,
        path: &str,
        content: FileContent,
        mime: &str,
    ) -> Result<FileRecord, FilesError> {
        let path = PagePath::parse(path)?;
        if !actor.scope.can_write(&path) {
            return Err(FilesError::Forbidden);
        }
        let mime = AllowedMime::parse(mime).ok_or(FilesError::UnsupportedMime)?;
        let bytes = content.into_bytes();
        self.check_size(&bytes)?;
        self.ensure_path_free(&path).await?;
        let pages = extract_pages(&mime, &bytes).await?;
        let sha256 = Sha256Hash::of(&bytes);
        let blob_key = BlobKey::from(&sha256);
        let size = i64::try_from(bytes.len()).map_err(|_| self.too_large())?;
        let mut tx = self.pool.begin().await?;
        queries::lock_blob(&mut tx, blob_key.as_str()).await?;
        if queries::path_is_taken(&mut tx, &path).await? {
            return Err(FilesError::PathTaken);
        }
        // Held under the blob lock so a purge of identical bytes cannot delete the blob before this row exists
        storage::write_blob(&*self.store, &blob_key, bytes).await?;
        let new_file = NewFile {
            path: &path,
            sha256: &sha256,
            blob_key: blob_key.as_str(),
            mime: mime.as_str(),
            size,
            user_id: actor.user_id,
        };
        let (id, created_at) = queries::insert_file(&mut tx, &new_file)
            .await
            .map_err(path_taken_or_database)?;
        queries::insert_pages(&mut tx, id, &pages).await?;
        tx.commit().await?;
        Ok(FileRecord {
            id,
            path,
            sha256,
            mime: mime.as_str().to_owned(),
            size,
            uploader: actor.user_id,
            created_at,
            page_count: i64::try_from(pages.len()).unwrap_or(i64::MAX),
        })
    }

    fn check_size(&self, bytes: &Bytes) -> Result<(), FilesError> {
        if bytes.is_empty() {
            return Err(FilesError::Empty);
        }
        if u64::try_from(bytes.len()).is_ok_and(|size| size <= self.max_upload_bytes) {
            Ok(())
        } else {
            Err(self.too_large())
        }
    }

    fn too_large(&self) -> FilesError {
        FilesError::TooLarge {
            limit: self.max_upload_bytes,
        }
    }

    async fn ensure_path_free(&self, path: &PagePath) -> Result<(), FilesError> {
        let mut conn = self.pool.acquire().await?;
        if queries::path_is_taken(&mut conn, path).await? {
            return Err(FilesError::PathTaken);
        }
        Ok(())
    }
}

async fn extract_pages(mime: &AllowedMime, bytes: &Bytes) -> Result<Vec<String>, FilesError> {
    match mime.kind() {
        FileKind::Pdf => Ok(pdf::extract_pages(bytes.clone()).await?),
        FileKind::Text => {
            let text = std::str::from_utf8(bytes).map_err(|_| FilesError::InvalidText)?;
            if let Some(pattern) = scan_secrets(text) {
                return Err(FilesError::Credential(pattern));
            }
            Ok(vec![text.replace('\0', "")])
        }
        FileKind::Image | FileKind::Spreadsheet | FileKind::Audio => Ok(Vec::new()),
    }
}

fn path_taken_or_database(error: sqlx::Error) -> FilesError {
    let is_path_conflict = error
        .as_database_error()
        .and_then(|database| database.constraint())
        == Some(PATH_LIVE_KEY);
    if is_path_conflict {
        FilesError::PathTaken
    } else {
        FilesError::Database(error)
    }
}
