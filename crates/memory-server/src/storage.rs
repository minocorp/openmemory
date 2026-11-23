mod blob_key;
mod config;
mod secret;
mod sha256_hash;

use std::sync::Arc;

use bytes::Bytes;
use object_store::aws::AmazonS3Builder;
use object_store::local::LocalFileSystem;
use object_store::{ObjectStore, ObjectStoreExt, PutPayload};

pub(crate) use blob_key::BlobKey;
pub(crate) use config::StorageConfig;
pub(crate) use sha256_hash::Sha256Hash;

#[derive(Debug, thiserror::Error)]
pub(crate) enum StorageError {
    #[error("create blob directory")]
    CreateDirectory(#[source] std::io::Error),
    #[error("configure object store")]
    Configure(#[source] object_store::Error),
}

pub(crate) async fn build(config: &StorageConfig) -> Result<Arc<dyn ObjectStore>, StorageError> {
    if let Some(s3) = config.s3() {
        let store = AmazonS3Builder::new()
            .with_endpoint(s3.endpoint)
            .with_bucket_name(s3.bucket)
            .with_region(s3.region)
            .with_access_key_id(s3.access_key)
            .with_secret_access_key(s3.secret_key.expose())
            .with_allow_http(s3.endpoint.starts_with("http://"))
            .build()
            .map_err(StorageError::Configure)?;
        return Ok(Arc::new(store));
    }
    tokio::fs::create_dir_all(config.blob_dir())
        .await
        .map_err(StorageError::CreateDirectory)?;
    let store =
        LocalFileSystem::new_with_prefix(config.blob_dir()).map_err(StorageError::Configure)?;
    Ok(Arc::new(store))
}

pub(crate) async fn write_blob(
    store: &dyn ObjectStore,
    key: &BlobKey,
    bytes: Bytes,
) -> Result<(), object_store::Error> {
    store.put(&key.to_path(), PutPayload::from(bytes)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::config::parse_for_test;

    #[tokio::test]
    async fn local_store_round_trips_a_blob_by_content_key() {
        let directory = tempfile::tempdir().unwrap();
        let blob_dir = directory.path().join("nested/blobs");
        let config = parse_for_test(&[&format!("--blob-dir={}", blob_dir.display())]).unwrap();
        let store = build(&config).await.unwrap();
        let key = BlobKey::from(&Sha256Hash::of(b"hello"));
        write_blob(&*store, &key, Bytes::from_static(b"hello"))
            .await
            .unwrap();
        let stored = store
            .get(&key.to_path())
            .await
            .unwrap()
            .bytes()
            .await
            .unwrap();
        assert_eq!(stored.as_ref(), b"hello");
    }

    #[tokio::test]
    async fn s3_store_builds_from_endpoint_and_credentials() {
        let config = parse_for_test(&[
            "--s3-endpoint=http://127.0.0.1:9000",
            "--s3-bucket=memory",
            "--s3-access-key=key",
            "--s3-secret-key=secret",
        ])
        .unwrap();
        assert!(build(&config).await.is_ok());
    }
}
