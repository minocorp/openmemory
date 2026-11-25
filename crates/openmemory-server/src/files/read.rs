use std::ops::Range;

use futures_util::StreamExt;
use object_store::{GetOptions, ObjectStore};

use crate::access::Actor;
use crate::files::queries;
use crate::files::{FileDownload, FileId, FileRecord, FileRef, Files, FilesError};
use crate::pages::scope_filter::ScopeFilter;
use crate::purge::{self, Subject};
use crate::storage::BlobKey;

impl Files {
    pub(crate) async fn metadata(
        &self,
        actor: &Actor,
        file: &FileRef,
    ) -> Result<FileRecord, FilesError> {
        let mut conn = self.pool.acquire().await?;
        let row = match file {
            FileRef::Id(id) => queries::find_live_by_id(&mut conn, *id).await?,
            FileRef::Path(path) => queries::find_live_by_path(&mut conn, path).await?,
        };
        let Some(row) = row else {
            if let FileRef::Id(id) = file
                && purge::is_purged(&mut conn, Subject::File, id.0).await?
            {
                return Err(FilesError::Purged);
            }
            return Err(FilesError::NotFound);
        };
        let record = FileRecord::try_from(row)?;
        if !actor.scope.can_read(&record.path) {
            return Err(FilesError::NotFound);
        }
        Ok(record)
    }

    pub(crate) async fn ids_for_paths(
        &self,
        paths: &[String],
    ) -> Result<Vec<(String, FileId)>, FilesError> {
        let mut conn = self.pool.acquire().await?;
        Ok(queries::ids_for_paths(&mut conn, paths).await?)
    }

    pub(crate) async fn list(&self, actor: &Actor) -> Result<Vec<FileRecord>, FilesError> {
        let mut conn = self.pool.acquire().await?;
        queries::list_live(&mut conn, &ScopeFilter::of(&actor.scope))
            .await?
            .into_iter()
            .map(FileRecord::try_from)
            .collect()
    }

    pub(crate) async fn page_text(
        &self,
        actor: &Actor,
        file: &FileRef,
        page: u32,
    ) -> Result<String, FilesError> {
        let record = self.metadata(actor, file).await?;
        let number = i32::try_from(page).map_err(|_| FilesError::PageOutOfRange { page })?;
        let mut conn = self.pool.acquire().await?;
        queries::page_text(&mut conn, record.id, number)
            .await?
            .ok_or(FilesError::PageOutOfRange { page })
    }

    pub(crate) async fn download(
        &self,
        actor: &Actor,
        file: &FileRef,
    ) -> Result<FileDownload, FilesError> {
        let record = self.metadata(actor, file).await?;
        self.open(record, None).await
    }

    pub(crate) async fn download_range(
        &self,
        actor: &Actor,
        file: &FileRef,
        range: Range<u64>,
    ) -> Result<FileDownload, FilesError> {
        let record = self.metadata(actor, file).await?;
        self.open(record, Some(range)).await
    }

    async fn open(
        &self,
        record: FileRecord,
        range: Option<Range<u64>>,
    ) -> Result<FileDownload, FilesError> {
        let options = GetOptions::new().with_range(range);
        let result = self
            .store
            .get_opts(&BlobKey::from(&record.sha256).to_path(), options)
            .await?;
        Ok(FileDownload {
            record,
            stream: result.into_stream().boxed(),
        })
    }

    pub(crate) async fn delete(&self, actor: &Actor, file: &FileRef) -> Result<(), FilesError> {
        let record = self.metadata(actor, file).await?;
        if !actor.scope.can_write(&record.path) {
            return Err(FilesError::Forbidden);
        }
        let mut conn = self.pool.acquire().await?;
        match queries::soft_delete(&mut conn, record.id).await? {
            0 => Err(FilesError::NotFound),
            _ => Ok(()),
        }
    }
}
