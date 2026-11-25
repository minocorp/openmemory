use futures_util::TryStreamExt;
use object_store::path::Path;
use object_store::{ObjectStore, ObjectStoreExt};
use sqlx::PgPool;
use time::OffsetDateTime;

use crate::files;

#[cfg(test)]
mod tests;

const BATCH: usize = 500;

#[derive(Debug, thiserror::Error)]
pub(crate) enum BlobGcError {
    #[error("blob storage failed")]
    Storage(#[from] object_store::Error),
    #[error("database query failed")]
    Database(#[from] sqlx::Error),
}

pub(super) async fn collect(
    pool: &PgPool,
    store: &dyn ObjectStore,
    older_than: OffsetDateTime,
) -> Result<usize, BlobGcError> {
    let cutoff = older_than.unix_timestamp();
    let candidates: Vec<Path> = store
        .list(None)
        .try_filter(|meta| std::future::ready(meta.last_modified.timestamp() < cutoff))
        .map_ok(|meta| meta.location)
        .try_collect()
        .await?;
    let mut removed = 0;
    for batch in candidates.chunks(BATCH) {
        let keys: Vec<String> = batch.iter().map(ToString::to_string).collect();
        let mut conn = pool.acquire().await?;
        let referenced = files::referenced_blob_keys(&mut conn, &keys).await?;
        drop(conn);
        for (path, key) in batch.iter().zip(&keys) {
            if !referenced.contains(key) && is_still_old(store, path, cutoff).await? {
                store.delete(path).await?;
                removed += 1;
            }
        }
    }
    Ok(removed)
}

async fn is_still_old(
    store: &dyn ObjectStore,
    path: &Path,
    cutoff: i64,
) -> Result<bool, object_store::Error> {
    // An identical re-upload rewrites the blob, which refreshes its age
    match store.head(path).await {
        Ok(meta) => Ok(meta.last_modified.timestamp() < cutoff),
        Err(object_store::Error::NotFound { .. }) => Ok(false),
        Err(error) => Err(error),
    }
}
