use std::sync::Arc;

use bytes::Bytes;
use futures_util::TryStreamExt;
use object_store::memory::InMemory;
use object_store::{ObjectStore, ObjectStoreExt};
use openmemory_ops::{CredentialPattern, PagePath};
use sqlx::PgPool;

use crate::access::{Actor, ConnectionId, Folders, Scope, UserId};
use crate::files::{FileContent, FileRef, Files, FilesError};
use crate::storage::{BlobKey, Sha256Hash};

const KOREAN_PDF: &[u8] = include_bytes!("../../tests/fixtures/korean.pdf");
const MAX_UPLOAD_BYTES: u64 = 1024 * 1024;

struct Fixture {
    files: Files,
    store: Arc<InMemory>,
    pool: PgPool,
    actor: Actor,
}

fn pdftotext_is_installed() -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|directory| directory.join("pdftotext").is_file())
    })
}

fn path(value: &str) -> PagePath {
    PagePath::parse(value).unwrap()
}

fn text(value: &str) -> FileContent {
    FileContent::Text(value.to_owned())
}

async fn fixture(pool: PgPool) -> Fixture {
    let user_id = sqlx::query_scalar!(r#"INSERT INTO users (username, display_name, user_handle) VALUES ('mino', 'Mino', uuid_send(gen_random_uuid())) RETURNING id"#)
        .fetch_one(&pool)
        .await
        .unwrap();
    let connection_id = insert_connection(&pool, user_id, "claude-code", "*").await;
    let store = Arc::new(InMemory::new());
    let files = Files::new(
        pool.clone(),
        Arc::<InMemory>::clone(&store),
        MAX_UPLOAD_BYTES,
    );
    let actor = Actor {
        user_id: UserId(user_id),
        connection_id: Some(ConnectionId(connection_id)),
        name: "mino/claude-code".to_owned(),
        scope: Scope::everything(),
    };
    Fixture {
        files,
        store,
        pool,
        actor,
    }
}

async fn insert_connection(pool: &PgPool, user_id: i64, name: &str, scope: &str) -> i64 {
    sqlx::query_scalar!(
        "INSERT INTO connections (user_id, name, scopes) VALUES ($1, $2, ARRAY[$3]) RETURNING id",
        user_id,
        name,
        scope,
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn insert_page(pool: &PgPool, page_path: &str) {
    sqlx::query!(
        "INSERT INTO pages (path, current_version, content) VALUES ($1, 1, 'x')",
        page_path,
    )
    .execute(pool)
    .await
    .unwrap();
}

#[sqlx::test]
async fn uploading_text_stores_the_blob_before_a_row_with_one_page(pool: PgPool) {
    let fx = fixture(pool).await;
    let record = fx
        .files
        .upload(
            &fx.actor,
            "notes/a.txt",
            text("hello\0 world"),
            "text/plain",
        )
        .await
        .unwrap();
    assert_eq!(record.size, 12);
    assert_eq!(record.page_count, 1);
    assert_eq!(record.mime, "text/plain");
    let key = BlobKey::from(&Sha256Hash::of(b"hello\0 world"));
    assert!(fx.store.head(&key.to_path()).await.is_ok());
    let page = fx
        .files
        .page_text(&fx.actor, &FileRef::Path(path("notes/a.txt")), 1)
        .await
        .unwrap();
    assert_eq!(page, "hello world");
}

#[sqlx::test]
async fn text_with_a_secret_is_rejected_before_anything_is_stored(pool: PgPool) {
    let fx = fixture(pool).await;
    let body = concat!("key AKIA", "ABCDEFGHIJKLMNOP");
    let result = fx
        .files
        .upload(&fx.actor, "notes/keys.txt", text(body), "text/plain")
        .await;
    assert!(matches!(
        result,
        Err(FilesError::Credential(CredentialPattern::AwsAccessKey))
    ));
    let key = BlobKey::from(&Sha256Hash::of(body.as_bytes()));
    assert!(fx.store.head(&key.to_path()).await.is_err());
    let found = fx
        .files
        .metadata(&fx.actor, &FileRef::Path(path("notes/keys.txt")))
        .await;
    assert!(matches!(found, Err(FilesError::NotFound)));
}

#[sqlx::test]
async fn personal_data_is_allowed_in_uploads(pool: PgPool) {
    let fx = fixture(pool).await;
    let evidence = "rrn 900101-1234568 card 4111 1111 1111 1111";
    fx.files
        .upload(&fx.actor, "tax/a.txt", text(evidence), "text/plain")
        .await
        .unwrap();
    fx.files
        .upload(
            &fx.actor,
            "tax/b.png",
            FileContent::Bytes(Bytes::from(evidence)),
            "image/png",
        )
        .await
        .unwrap();
}

#[sqlx::test]
async fn uploading_a_pdf_extracts_korean_text_per_page(pool: PgPool) {
    if !pdftotext_is_installed() {
        return;
    }
    let fx = fixture(pool).await;
    let record = fx
        .files
        .upload(
            &fx.actor,
            "contracts/acme.pdf",
            FileContent::Bytes(Bytes::from_static(KOREAN_PDF)),
            "application/pdf",
        )
        .await
        .unwrap();
    assert_eq!(record.page_count, 2);
    let file = FileRef::Id(record.id);
    let first = fx.files.page_text(&fx.actor, &file, 1).await.unwrap();
    let second = fx.files.page_text(&fx.actor, &file, 2).await.unwrap();
    assert!(first.contains("오십만원정"), "{first:?}");
    assert!(second.contains("계약서"), "{second:?}");
    assert!(matches!(
        fx.files.page_text(&fx.actor, &file, 3).await,
        Err(FilesError::PageOutOfRange { page: 3 })
    ));
}

#[sqlx::test]
async fn a_broken_pdf_is_rejected_without_a_row(pool: PgPool) {
    if !pdftotext_is_installed() {
        return;
    }
    let fx = fixture(pool).await;
    let result = fx
        .files
        .upload(
            &fx.actor,
            "broken.pdf",
            FileContent::Bytes(Bytes::from_static(b"not a pdf")),
            "application/pdf",
        )
        .await;
    assert!(matches!(result, Err(FilesError::Pdf(_))));
    let metadata = fx
        .files
        .metadata(&fx.actor, &FileRef::Path(path("broken.pdf")))
        .await;
    assert!(matches!(metadata, Err(FilesError::NotFound)));
}

#[sqlx::test]
async fn images_are_stored_without_page_text(pool: PgPool) {
    let fx = fixture(pool).await;
    let record = fx
        .files
        .upload(
            &fx.actor,
            "img/logo.png",
            FileContent::Bytes(Bytes::from_static(b"\x89PNG")),
            "image/png",
        )
        .await
        .unwrap();
    assert_eq!(record.page_count, 0);
}

#[sqlx::test]
async fn a_path_held_by_a_live_page_is_rejected(pool: PgPool) {
    let fx = fixture(pool).await;
    insert_page(&fx.pool, "customers/acme").await;
    let result = fx
        .files
        .upload(&fx.actor, "customers/acme", text("x"), "text/plain")
        .await;
    assert!(matches!(result, Err(FilesError::PathTaken)));
}

#[sqlx::test]
async fn a_path_held_by_a_live_file_is_rejected_until_it_is_deleted(pool: PgPool) {
    let fx = fixture(pool).await;
    let record = fx
        .files
        .upload(&fx.actor, "a.txt", text("one"), "text/plain")
        .await
        .unwrap();
    let second = fx
        .files
        .upload(&fx.actor, "a.txt", text("two"), "text/plain")
        .await;
    assert!(matches!(second, Err(FilesError::PathTaken)));
    fx.files
        .delete(&fx.actor, &FileRef::Id(record.id))
        .await
        .unwrap();
    assert!(
        fx.files
            .upload(&fx.actor, "a.txt", text("two"), "text/plain")
            .await
            .is_ok()
    );
}

#[sqlx::test]
async fn invalid_uploads_are_rejected_by_cause(pool: PgPool) {
    let fx = fixture(pool).await;
    let too_big = FileContent::Bytes(Bytes::from(vec![b'a'; 1024 * 1024 + 1]));
    let cases = [
        ("a/../b", text("x"), "text/plain"),
        ("a.html", text("x"), "text/html"),
        ("a.txt", text(""), "text/plain"),
        ("a.txt", too_big, "text/plain"),
        (
            "a.txt",
            FileContent::Bytes(Bytes::from_static(&[0xff, 0xfe])),
            "text/plain",
        ),
    ];
    let mut errors = Vec::new();
    for (target, content, mime) in cases {
        errors.push(
            fx.files
                .upload(&fx.actor, target, content, mime)
                .await
                .unwrap_err(),
        );
    }
    assert!(matches!(errors[0], FilesError::InvalidPath(_)));
    assert!(matches!(errors[1], FilesError::UnsupportedMime));
    assert!(matches!(errors[2], FilesError::Empty));
    assert!(matches!(errors[3], FilesError::TooLarge { .. }));
    assert!(matches!(errors[4], FilesError::InvalidText));
}

#[sqlx::test]
async fn scope_limits_who_can_write_and_read(pool: PgPool) {
    let fx = fixture(pool).await;
    let tax_only = Actor {
        scope: Scope {
            folders: Folders::Only(vec![path("tax")]),
            can_write: true,
        },
        ..fx.actor.clone()
    };
    let read_only = Actor {
        scope: Scope {
            can_write: false,
            ..Scope::everything()
        },
        ..fx.actor.clone()
    };
    let outside = fx
        .files
        .upload(&tax_only, "legal/a.txt", text("x"), "text/plain")
        .await;
    assert!(matches!(outside, Err(FilesError::Forbidden)));
    fx.files
        .upload(&tax_only, "tax/a.txt", text("x"), "text/plain")
        .await
        .unwrap();
    fx.files
        .upload(&fx.actor, "legal/b.txt", text("x"), "text/plain")
        .await
        .unwrap();
    let hidden = fx
        .files
        .metadata(&tax_only, &FileRef::Path(path("legal/b.txt")))
        .await;
    assert!(matches!(hidden, Err(FilesError::NotFound)));
    let write = fx
        .files
        .upload(&read_only, "c.txt", text("x"), "text/plain")
        .await;
    assert!(matches!(write, Err(FilesError::Forbidden)));
    let delete = fx
        .files
        .delete(&read_only, &FileRef::Path(path("legal/b.txt")))
        .await;
    assert!(matches!(delete, Err(FilesError::Forbidden)));
}

#[sqlx::test]
async fn download_streams_the_stored_bytes(pool: PgPool) {
    let fx = fixture(pool).await;
    fx.files
        .upload(&fx.actor, "a.txt", text("stored body"), "text/plain")
        .await
        .unwrap();
    let download = fx
        .files
        .download(&fx.actor, &FileRef::Path(path("a.txt")))
        .await
        .unwrap();
    let chunks: Vec<Bytes> = download.stream.try_collect().await.unwrap();
    assert_eq!(chunks.concat(), b"stored body");
}

#[sqlx::test]
async fn identical_content_shares_one_blob(pool: PgPool) {
    let fx = fixture(pool).await;
    for target in ["a.txt", "b.txt"] {
        fx.files
            .upload(&fx.actor, target, text("same"), "text/plain")
            .await
            .unwrap();
    }
    let blobs: Vec<_> = fx.store.list(None).try_collect().await.unwrap();
    assert_eq!(blobs.len(), 1);
}

mod links;
mod purge;
