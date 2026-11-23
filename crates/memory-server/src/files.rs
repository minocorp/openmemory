mod error;
mod file_id;
mod folder;
mod folder_queries;
mod link_queries;
mod links;
mod mime;
mod pdf;
mod purge;
mod queries;
mod read;
mod record;
mod upload;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use object_store::ObjectStore;
use sqlx::PgPool;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

pub(crate) use error::FilesError;
pub(crate) use file_id::FileId;
pub(crate) use mime::mime_for_path;
pub(crate) use queries::referenced_blob_keys;
pub(crate) use record::{FileContent, FileDownload, FileRecord, FileRef};

const MAX_CONCURRENT_UPLOAD_BODIES: usize = 4;

#[derive(Clone)]
pub(crate) struct Files {
    pool: PgPool,
    store: Arc<dyn ObjectStore>,
    max_upload_bytes: u64,
    upload_bodies: Arc<Semaphore>,
}

impl Files {
    pub(crate) fn new(pool: PgPool, store: Arc<dyn ObjectStore>, max_upload_bytes: u64) -> Self {
        Self {
            pool,
            store,
            max_upload_bytes,
            upload_bodies: Arc::new(Semaphore::new(MAX_CONCURRENT_UPLOAD_BODIES)),
        }
    }

    pub(crate) fn reserve_upload_body(&self) -> Option<OwnedSemaphorePermit> {
        Arc::clone(&self.upload_bodies).try_acquire_owned().ok()
    }

    pub(crate) fn max_upload_bytes(&self) -> u64 {
        self.max_upload_bytes
    }
}
