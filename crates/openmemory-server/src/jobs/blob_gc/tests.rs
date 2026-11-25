use std::sync::Arc;

use bytes::Bytes;
use object_store::ObjectStoreExt;
use object_store::memory::InMemory;
use sqlx::PgPool;
use time::{Duration, OffsetDateTime};

use super::collect;
use crate::access::{Actor, Scope, UserId};
use crate::files::{FileContent, Files};
use crate::storage::{BlobKey, Sha256Hash};

async fn store_with_one_file_and_one_orphan(pool: &PgPool) -> (Arc<InMemory>, BlobKey, BlobKey) {
    let user_id = sqlx::query_scalar!("INSERT INTO users (username, display_name, user_handle) VALUES ('mino', 'Mino', uuid_send(gen_random_uuid())) RETURNING id")
        .fetch_one(pool)
        .await
        .unwrap();
    let store = Arc::new(InMemory::new());
    let files = Files::new(pool.clone(), Arc::<InMemory>::clone(&store), 1024);
    let actor = Actor {
        user_id: UserId(user_id),
        connection_id: None,
        name: "mino".to_owned(),
        scope: Scope::everything(),
    };
    files
        .upload(
            &actor,
            "kept.txt",
            FileContent::Text("kept".to_owned()),
            "text/plain",
        )
        .await
        .unwrap();
    let kept = BlobKey::from(&Sha256Hash::of(b"kept"));
    let orphan = BlobKey::from(&Sha256Hash::of(b"orphan"));
    crate::storage::write_blob(&*store, &orphan, Bytes::from_static(b"orphan"))
        .await
        .unwrap();
    (store, kept, orphan)
}

#[sqlx::test]
async fn old_unreferenced_blobs_are_removed_and_referenced_ones_kept(pool: PgPool) {
    let (store, kept, orphan) = store_with_one_file_and_one_orphan(&pool).await;
    let removed = collect(
        &pool,
        &*store,
        OffsetDateTime::now_utc() + Duration::hours(1),
    )
    .await
    .unwrap();
    assert_eq!(removed, 1);
    assert!(store.head(&kept.to_path()).await.is_ok());
    assert!(store.head(&orphan.to_path()).await.is_err());
}

#[sqlx::test]
async fn recent_unreferenced_blobs_are_left_for_a_later_run(pool: PgPool) {
    let (store, _, orphan) = store_with_one_file_and_one_orphan(&pool).await;
    let removed = collect(
        &pool,
        &*store,
        OffsetDateTime::now_utc() - Duration::hours(1),
    )
    .await
    .unwrap();
    assert_eq!(removed, 0);
    assert!(store.head(&orphan.to_path()).await.is_ok());
}

#[sqlx::test]
async fn blobs_of_soft_deleted_files_are_kept(pool: PgPool) {
    let (store, kept, _) = store_with_one_file_and_one_orphan(&pool).await;
    sqlx::query!("UPDATE files SET deleted_at = now()")
        .execute(&pool)
        .await
        .unwrap();
    collect(
        &pool,
        &*store,
        OffsetDateTime::now_utc() + Duration::hours(1),
    )
    .await
    .unwrap();
    assert!(store.head(&kept.to_path()).await.is_ok());
}
