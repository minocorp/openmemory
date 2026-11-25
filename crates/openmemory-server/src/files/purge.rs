use object_store::ObjectStoreExt;
use object_store::path::Path;

use crate::access::UserId;
use crate::files::queries;
use crate::files::{FileId, Files, FilesError};
use crate::purge::{self, Subject};

impl Files {
    pub(crate) async fn purge(&self, user_id: UserId, id: FileId) -> Result<(), FilesError> {
        let mut tx = self.pool.begin().await?;
        let file = queries::lock_any_by_id(&mut tx, id)
            .await?
            .ok_or(FilesError::NotFound)?;
        queries::lock_blob(&mut tx, &file.blob_key).await?;
        purge::record(&mut tx, Subject::File, id.0, &file.path, user_id).await?;
        queries::delete_by_id(&mut tx, id).await?;
        if !queries::blob_is_referenced(&mut tx, &file.blob_key).await? {
            // The blob lock must outlive the delete, or a concurrent identical upload could store a row for a deleted blob
            self.remove_blob(&file.blob_key).await?;
        }
        tx.commit().await?;
        Ok(())
    }

    async fn remove_blob(&self, blob_key: &str) -> Result<(), FilesError> {
        let path = Path::from(blob_key);
        match self.store.delete(&path).await {
            Ok(()) | Err(object_store::Error::NotFound { .. }) => Ok(()),
            Err(error) => Err(FilesError::Storage(error)),
        }
    }
}
