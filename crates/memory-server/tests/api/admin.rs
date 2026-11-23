use std::io::{Cursor, Read, Write};

use axum::http::{Method, StatusCode};
use serde_json::json;
use sqlx::PgPool;
use zip::write::SimpleFileOptions;

use crate::pages::{PAGE, created_id};
use crate::support::{App, Browser, TestResult};

fn archive(entries: &[(&str, &[u8])]) -> TestResult<Vec<u8>> {
    let mut writer = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, data) in entries {
        writer.start_file(*name, SimpleFileOptions::default())?;
        writer.write_all(data)?;
    }
    Ok(writer.finish()?.into_inner())
}

fn entries(bytes: &[u8]) -> TestResult<Vec<(String, Vec<u8>)>> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes))?;
    let mut found = Vec::new();
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let mut data = Vec::new();
        entry.read_to_end(&mut data)?;
        found.push((entry.name()?.into_owned(), data));
    }
    found.sort();
    Ok(found)
}

async fn import(browser: &Browser, bytes: Vec<u8>) -> TestResult<crate::support::Raw> {
    browser
        .raw(
            Method::POST,
            "/api/system/import",
            &[("content-type", "application/zip")],
            bytes,
        )
        .await
}

#[sqlx::test]
async fn admins_list_users_and_change_admin_flags(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let member = app.member(&admin, "member").await?;

    let users = admin.get("/api/users").await?;
    assert_eq!(users.status, StatusCode::OK, "{:?}", users.body);
    let listed = users.body.as_array().ok_or("no users")?;
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0]["username"], "admin");
    assert_eq!(listed[0]["display_name"], "Display admin");
    assert_eq!(listed[0]["is_admin"], true);
    assert_eq!(listed[1]["is_admin"], false);
    let member_id = listed[1]["id"].as_i64().ok_or("no id")?;

    assert_eq!(
        member.get("/api/users").await?.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        member.set_admin(member_id, true).await?.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.browser().get("/api/users").await?.status,
        StatusCode::UNAUTHORIZED
    );

    let promoted = admin.set_admin(member_id, true).await?;
    assert_eq!(promoted.status, StatusCode::OK);
    assert_eq!(promoted.body["is_admin"], true);
    assert_eq!(member.get("/api/users").await?.status, StatusCode::OK);

    let unknown = admin.set_admin(999, true).await?;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);
    Ok(())
}

#[sqlx::test]
async fn the_last_admin_cannot_be_demoted(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let me = admin.get("/api/auth/me").await?.body["id"]
        .as_i64()
        .ok_or("no id")?;
    let blocked = admin.set_admin(me, false).await?;
    assert_eq!(blocked.status, StatusCode::CONFLICT);
    assert_eq!(blocked.body["error"], "last_admin");

    let member = app.member(&admin, "member").await?;
    let member_id = member.get("/api/auth/me").await?.body["id"]
        .as_i64()
        .ok_or("no id")?;
    admin.set_admin(member_id, true).await?;
    let demoted = admin.set_admin(me, false).await?;
    assert_eq!(demoted.status, StatusCode::OK);
    Ok(())
}

#[sqlx::test]
async fn the_system_view_reports_the_database_and_leaves_unmonitored_backups_empty(
    pool: PgPool,
) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let member = app.member(&admin, "member").await?;
    let system = admin.get("/api/system").await?;
    assert_eq!(system.status, StatusCode::OK, "{:?}", system.body);
    assert_eq!(system.body["database"], "ok");
    assert!(system.body["backup"].is_null());
    assert_eq!(
        member.get("/api/system").await?.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.browser().get("/api/system").await?.status,
        StatusCode::UNAUTHORIZED
    );
    Ok(())
}

#[sqlx::test]
async fn export_and_import_round_trip_pages_and_files(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let acme = created_id(&admin, "clients/acme", PAGE).await?;
    let hub = created_id(
        &admin,
        "notes/hub",
        "See [[clients/acme]] and [[files/terms.txt]].\n",
    )
    .await?;
    let uploaded = admin
        .raw(
            Method::POST,
            "/api/files?path=files/terms.txt",
            &[("content-type", "text/plain")],
            b"net 30\n".to_vec(),
        )
        .await?;
    assert_eq!(uploaded.status, StatusCode::CREATED);
    let file_id = serde_json::from_slice::<serde_json::Value>(&uploaded.body)?["id"]
        .as_i64()
        .ok_or("no id")?;

    let raw = admin.export().await?;
    assert_eq!(raw.status, StatusCode::OK);
    assert_eq!(raw.header("content-type"), "application/zip");
    assert!(
        raw.header("content-disposition")
            .contains("memory-export.zip")
    );
    let listed = entries(&raw.body)?;
    let names: Vec<&str> = listed.iter().map(|(name, _)| name.as_str()).collect();
    assert_eq!(
        names,
        [
            "files/files/terms.txt",
            "pages/clients/acme.md",
            "pages/notes/hub.md"
        ]
    );
    assert_eq!(listed[0].1, b"net 30\n");
    assert_eq!(listed[1].1, PAGE.as_bytes());

    for path in [format!("/api/pages/{acme}"), format!("/api/pages/{hub}")] {
        let version = admin.get(&path).await?.body["version"]
            .as_i64()
            .ok_or("no version")?;
        let deleted = admin
            .exchange(
                Method::DELETE,
                &path,
                &[("if-match", &format!("\"{version}\""))],
                None,
            )
            .await?;
        assert_eq!(deleted.status, StatusCode::NO_CONTENT);
    }
    admin.delete(&format!("/api/files/{file_id}")).await?;

    let report = import(&admin, raw.body.to_vec()).await?;
    assert_eq!(report.status, StatusCode::OK);
    let report: serde_json::Value = serde_json::from_slice(&report.body)?;
    assert_eq!(report["pages"], 2);
    assert_eq!(report["files"], 1);
    assert_eq!(report["skipped"], json!([]));

    let restored = admin.get("/api/pages?path=clients/acme").await?;
    assert_eq!(restored.body["content"], PAGE);
    assert_eq!(restored.body["author"], "import");
    assert_ne!(restored.body["id"], acme, "an import creates new pages");
    let linker = admin.get("/api/pages?path=notes/hub").await?;
    assert_eq!(
        linker.body["content"],
        "See [[clients/acme]] and [[files/terms.txt]].\n"
    );
    let file = admin.get("/api/files?path=files/terms.txt").await?;
    assert_eq!(file.body["size"], 7);
    Ok(())
}

#[sqlx::test]
async fn import_reports_what_it_skips_and_keeps_the_rest(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    created_id(&admin, "taken", "mine\n").await?;
    let bytes = archive(&[
        ("pages/fresh.md", b"hello\n"),
        ("pages/taken.md", b"theirs\n"),
        ("pages/broken.md", b"## Timeline\nnot a line\n"),
        ("pages/secret.md", b"card 4111 1111 1111 1111\n"),
        ("pages/a//b.md", b"x\n"),
        ("files/data.csv", b"a,b\n1,2\n"),
        ("files/tool.exe", b"MZ"),
        ("files/empty.txt", b""),
        ("elsewhere/readme.md", b"x\n"),
    ])?;
    let reply = import(&admin, bytes).await?;
    assert_eq!(reply.status, StatusCode::OK);
    let report: serde_json::Value = serde_json::from_slice(&reply.body)?;
    assert_eq!(report["pages"], 1);
    assert_eq!(report["files"], 1);
    let mut skipped: Vec<(String, String)> = report["skipped"]
        .as_array()
        .ok_or("no skipped")?
        .iter()
        .map(|item| {
            (
                item["name"].as_str().unwrap_or_default().to_owned(),
                item["reason"].as_str().unwrap_or_default().to_owned(),
            )
        })
        .collect();
    skipped.sort();
    let expected = [
        ("elsewhere/readme.md", "unrecognized"),
        ("files/empty.txt", "empty"),
        ("files/tool.exe", "unsupported_type"),
        ("pages/a//b.md", "invalid_path"),
        ("pages/broken.md", "invalid_page"),
        ("pages/secret.md", "credential"),
        ("pages/taken.md", "path_taken"),
    ];
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(name, reason)| ((*name).to_owned(), (*reason).to_owned()))
        .collect();
    assert_eq!(skipped, expected);
    assert_eq!(
        admin.get("/api/pages?path=taken").await?.body["content"],
        "mine\n"
    );
    assert_eq!(
        admin.get("/api/pages?path=fresh").await?.status,
        StatusCode::OK
    );
    Ok(())
}

#[sqlx::test]
async fn import_and_export_are_admin_only_and_validate_the_archive(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let member = app.member(&admin, "member").await?;
    let bytes = archive(&[("pages/a.md", b"x\n")])?;
    assert_eq!(
        import(&member, bytes.clone()).await?.status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(member.export().await?.status, StatusCode::FORBIDDEN);
    assert_eq!(
        import(&app.browser(), bytes).await?.status,
        StatusCode::UNAUTHORIZED
    );
    let garbage = import(&admin, b"not a zip".to_vec()).await?;
    assert_eq!(garbage.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        admin.get("/api/pages?path=a").await?.status,
        StatusCode::NOT_FOUND
    );
    Ok(())
}
