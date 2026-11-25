use std::io::{Cursor, Read, Write};
use std::sync::Arc;

use bytes::Bytes;
use futures_util::TryStreamExt;
use object_store::memory::InMemory;
use sqlx::PgPool;
use zip::write::SimpleFileOptions;

use super::*;
use crate::access::Actor;
use crate::files::{FileContent, FileRef, Files};
use crate::pages::read::read_with_id;
use crate::test_support::{agent, create, path, scoped};
use crate::time_zone::TimeZone;

const PAGE: &str = "---\nstatus: active\n---\nBody.\n\n## Timeline\n2026-03-02 | call — signed\n";

async fn fixture(pool: &PgPool) -> (Files, Actor) {
    let actor = agent(pool, "mino", "claude-code").await;
    let files = Files::new(pool.clone(), Arc::new(InMemory::new()), 1024 * 1024);
    (files, actor)
}

async fn upload(files: &Files, actor: &Actor, file_path: &str, body: &str) {
    files
        .upload(
            actor,
            file_path,
            FileContent::Text(body.to_owned()),
            "text/plain",
        )
        .await
        .unwrap();
}

async fn exported(pool: &PgPool, files: &Files, actor: &Actor) -> Vec<(String, Vec<u8>)> {
    let chunks: Vec<Bytes> = export(pool, files, actor)
        .await
        .unwrap()
        .try_collect()
        .await
        .unwrap();
    let mut zip = zip::ZipArchive::new(Cursor::new(chunks.concat())).unwrap();
    let mut found = Vec::new();
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index).unwrap();
        let mut data = Vec::new();
        entry.read_to_end(&mut data).unwrap();
        found.push((entry.name().unwrap().into_owned(), data));
    }
    found.sort();
    found
}

fn names(entries: &[(String, Vec<u8>)]) -> Vec<&str> {
    entries.iter().map(|(name, _)| name.as_str()).collect()
}

#[sqlx::test]
async fn export_writes_plain_markdown_and_original_files(pool: PgPool) {
    let (files, actor) = fixture(&pool).await;
    create(&pool, &actor, "clients/acme", PAGE).await;
    upload(&files, &actor, "clients/terms.txt", "net 30").await;
    let entries = exported(&pool, &files, &actor).await;
    assert_eq!(
        names(&entries),
        ["files/clients/terms.txt", "pages/clients/acme.md"]
    );
    assert_eq!(entries[0].1, b"net 30");
    assert_eq!(entries[1].1, PAGE.as_bytes());
}

#[sqlx::test]
async fn export_is_scoped_to_the_caller(pool: PgPool) {
    let (files, actor) = fixture(&pool).await;
    create(&pool, &actor, "clients/acme", PAGE).await;
    create(&pool, &actor, "tax/2026", "refund\n").await;
    upload(&files, &actor, "clients/terms.txt", "private").await;
    upload(&files, &actor, "tax/receipt.txt", "receipt").await;
    let tax_only = scoped(&actor, &["tax"], false);
    let entries = exported(&pool, &files, &tax_only).await;
    assert_eq!(
        names(&entries),
        ["files/tax/receipt.txt", "pages/tax/2026.md"]
    );
}

#[sqlx::test]
async fn export_skips_deleted_pages_and_files(pool: PgPool) {
    let (files, actor) = fixture(&pool).await;
    create(&pool, &actor, "a", "one\n").await;
    create(&pool, &actor, "b", "two\n").await;
    crate::pages::delete::delete(&pool, &actor, &path("b"), None)
        .await
        .unwrap();
    upload(&files, &actor, "gone.txt", "x").await;
    files
        .delete(&actor, &FileRef::Path(path("gone.txt")))
        .await
        .unwrap();
    let entries = exported(&pool, &files, &actor).await;
    assert_eq!(names(&entries), ["pages/a.md"]);
}

#[sqlx::test]
async fn an_empty_notebook_exports_an_empty_archive(pool: PgPool) {
    let (files, actor) = fixture(&pool).await;
    assert_eq!(
        exported(&pool, &files, &actor).await,
        Vec::<(String, Vec<u8>)>::new()
    );
}

fn archive(entries: &[(&str, &[u8])], method: zip::CompressionMethod) -> Bytes {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in entries {
        writer
            .start_file(
                *name,
                SimpleFileOptions::default().compression_method(method),
            )
            .unwrap();
        writer.write_all(data).unwrap();
    }
    Bytes::from(writer.finish().unwrap().into_inner())
}

#[sqlx::test]
async fn import_writes_through_the_normal_path_as_the_import_author(pool: PgPool) {
    let (files, actor) = fixture(&pool).await;
    let bytes = archive(
        &[
            ("pages/clients/acme.md", PAGE.as_bytes()),
            ("files/notes.txt", b"hello"),
        ],
        zip::CompressionMethod::Deflated,
    );
    let report = import(&pool, &TimeZone::utc(), &files, &actor, bytes)
        .await
        .unwrap();
    assert_eq!((report.pages, report.files), (1, 1));
    assert_eq!(report.skipped, Vec::new());

    let (_, page) = read_with_id(&pool, &actor, &path("clients/acme"), None)
        .await
        .unwrap();
    assert_eq!(page.author, "import");
    assert_eq!(page.content, PAGE);
    let task =
        sqlx::query_scalar!(r#"SELECT task AS "task!" FROM change_groups WHERE task IS NOT NULL"#)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(task, "import");
    let stored = files
        .metadata(&actor, &FileRef::Path(path("notes.txt")))
        .await
        .unwrap();
    assert_eq!(stored.size, 5);
}

#[sqlx::test]
async fn import_links_pages_to_files_that_arrive_in_the_same_archive(pool: PgPool) {
    let (files, actor) = fixture(&pool).await;
    let bytes = archive(
        &[
            ("pages/hub.md", b"See [[docs/terms.txt]].\n"),
            ("files/docs/terms.txt", b"net 30"),
        ],
        zip::CompressionMethod::Stored,
    );
    import(&pool, &TimeZone::utc(), &files, &actor, bytes)
        .await
        .unwrap();
    let links = sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM page_links WHERE to_file_id IS NOT NULL"#
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(links, 1);
}

#[sqlx::test]
async fn import_ignores_the_importers_scope_and_never_overwrites(pool: PgPool) {
    let (files, actor) = fixture(&pool).await;
    create(&pool, &actor, "taken", "mine\n").await;
    let narrow = scoped(&actor, &["tax"], false);
    let bytes = archive(
        &[
            ("pages/taken.md", b"theirs\n"),
            ("pages/other/new.md", b"new\n"),
        ],
        zip::CompressionMethod::Stored,
    );
    let report = import(&pool, &TimeZone::utc(), &files, &narrow, bytes)
        .await
        .unwrap();
    assert_eq!(report.pages, 1);
    assert_eq!(report.skipped.len(), 1);
    assert_eq!(report.skipped[0].reason, SkipReason::PathTaken);
    let (_, kept) = read_with_id(&pool, &actor, &path("taken"), None)
        .await
        .unwrap();
    assert_eq!(kept.content, "mine\n");
}

#[sqlx::test]
async fn a_corrupt_archive_is_rejected_whole(pool: PgPool) {
    let (files, actor) = fixture(&pool).await;
    let result = import(
        &pool,
        &TimeZone::utc(),
        &files,
        &actor,
        Bytes::from_static(b"definitely not a zip"),
    )
    .await;
    assert!(matches!(result, Err(TransferError::InvalidArchive(_))));
}
