use super::*;
use crate::access::Folders;
use crate::files::FileId;
use crate::files::queries;

async fn upload(fx: &Fixture, file_path: &str, body: &str) -> crate::files::FileRecord {
    fx.files
        .upload(&fx.actor, file_path, text(body), "text/plain")
        .await
        .unwrap()
}

fn blob_path(body: &str) -> object_store::path::Path {
    BlobKey::from(&Sha256Hash::of(body.as_bytes())).to_path()
}

#[sqlx::test]
async fn purging_removes_the_row_the_text_the_blob_and_leaves_a_tombstone(pool: PgPool) {
    let fx = fixture(pool).await;
    let record = upload(&fx, "tax/receipt.txt", "secret terms").await;
    assert!(fx.store.head(&blob_path("secret terms")).await.is_ok());

    fx.files.purge(fx.actor.user_id, record.id).await.unwrap();

    assert!(fx.store.head(&blob_path("secret terms")).await.is_err());
    let rows = sqlx::query_scalar!("SELECT count(*) AS \"count!\" FROM files")
        .fetch_one(&fx.pool)
        .await
        .unwrap();
    assert_eq!(rows, 0);
    let indexed = sqlx::query_scalar!("SELECT count(*) AS \"count!\" FROM file_pages")
        .fetch_one(&fx.pool)
        .await
        .unwrap();
    assert_eq!(indexed, 0);
    let tombstone = sqlx::query!("SELECT kind, subject_id, path, user_id FROM purges")
        .fetch_one(&fx.pool)
        .await
        .unwrap();
    assert_eq!(tombstone.kind, "file");
    assert_eq!(tombstone.subject_id, record.id.0);
    assert_eq!(tombstone.path, "tax/receipt.txt");
    assert_eq!(tombstone.user_id, fx.actor.user_id.0);
    let read = fx.files.metadata(&fx.actor, &FileRef::Id(record.id)).await;
    assert!(matches!(read, Err(FilesError::Purged)));
}

#[sqlx::test]
async fn a_blob_shared_with_another_file_survives_the_purge(pool: PgPool) {
    let fx = fixture(pool).await;
    let first = upload(&fx, "a.txt", "same bytes").await;
    let second = upload(&fx, "b.txt", "same bytes").await;

    fx.files.purge(fx.actor.user_id, first.id).await.unwrap();
    assert!(fx.store.head(&blob_path("same bytes")).await.is_ok());
    let download = fx
        .files
        .download(&fx.actor, &FileRef::Id(second.id))
        .await
        .unwrap();
    let chunks: Vec<Bytes> = download.stream.try_collect().await.unwrap();
    assert_eq!(chunks.concat(), b"same bytes");

    fx.files.purge(fx.actor.user_id, second.id).await.unwrap();
    assert!(fx.store.head(&blob_path("same bytes")).await.is_err());
}

#[sqlx::test]
async fn a_soft_deleted_file_can_still_be_purged(pool: PgPool) {
    let fx = fixture(pool).await;
    let record = upload(&fx, "a.txt", "gone soon").await;
    fx.files
        .delete(&fx.actor, &FileRef::Id(record.id))
        .await
        .unwrap();
    assert!(fx.store.head(&blob_path("gone soon")).await.is_ok());
    fx.files.purge(fx.actor.user_id, record.id).await.unwrap();
    assert!(fx.store.head(&blob_path("gone soon")).await.is_err());
}

#[sqlx::test]
async fn purging_an_unknown_file_is_not_found(pool: PgPool) {
    let fx = fixture(pool).await;
    let result = fx.files.purge(fx.actor.user_id, FileId(404)).await;
    assert!(matches!(result, Err(FilesError::NotFound)));
}

#[sqlx::test]
async fn listing_shows_only_live_files_inside_the_scope(pool: PgPool) {
    let fx = fixture(pool).await;
    upload(&fx, "tax/a.txt", "one").await;
    upload(&fx, "tax/b.txt", "two").await;
    upload(&fx, "clients/c.txt", "three").await;
    let removed = upload(&fx, "tax/gone.txt", "four").await;
    fx.files
        .delete(&fx.actor, &FileRef::Id(removed.id))
        .await
        .unwrap();

    let everything = fx.files.list(&fx.actor).await.unwrap();
    let paths: Vec<&str> = everything.iter().map(|file| file.path.as_str()).collect();
    assert_eq!(paths, ["clients/c.txt", "tax/a.txt", "tax/b.txt"]);

    let scoped = Actor {
        scope: Scope {
            folders: Folders::Only(vec![path("tax")]),
            can_write: true,
        },
        ..fx.actor.clone()
    };
    let visible = fx.files.list(&scoped).await.unwrap();
    let paths: Vec<&str> = visible.iter().map(|file| file.path.as_str()).collect();
    assert_eq!(paths, ["tax/a.txt", "tax/b.txt"]);
}

#[sqlx::test]
async fn a_purge_waits_for_an_upload_of_the_same_bytes_in_flight(pool: PgPool) {
    let fx = fixture(pool).await;
    let first = upload(&fx, "a.txt", "same bytes").await;
    let key = BlobKey::from(&Sha256Hash::of(b"same bytes"));
    let mut in_flight = fx.pool.begin().await.unwrap();
    queries::lock_blob(&mut in_flight, key.as_str())
        .await
        .unwrap();

    let waiting = tokio::time::timeout(
        std::time::Duration::from_millis(300),
        fx.files.purge(fx.actor.user_id, first.id),
    )
    .await;
    assert!(waiting.is_err());
    drop(in_flight);
    assert!(fx.store.head(&blob_path("same bytes")).await.is_ok());

    fx.files.purge(fx.actor.user_id, first.id).await.unwrap();
    assert!(fx.store.head(&blob_path("same bytes")).await.is_err());
}

#[sqlx::test]
async fn concurrent_purge_and_identical_upload_never_leave_a_row_without_its_blob(pool: PgPool) {
    let fx = fixture(pool).await;
    for round in 0..20 {
        let body = format!("round {round}");
        let old = upload(&fx, &format!("old/{round}.txt"), &body).await;
        let new_path = format!("new/{round}.txt");
        let (purged, uploaded) = tokio::join!(
            fx.files.purge(fx.actor.user_id, old.id),
            fx.files
                .upload(&fx.actor, &new_path, text(&body), "text/plain"),
        );
        purged.unwrap();
        let record = uploaded.unwrap();
        assert!(
            fx.store.head(&blob_path(&body)).await.is_ok(),
            "blob of {} is missing",
            record.path
        );
    }
}
