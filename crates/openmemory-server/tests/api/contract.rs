use axum::http::{HeaderMap, Method, StatusCode};
use serde_json::{Value, json};
use sqlx::PgPool;

use crate::pages::{PAGE, create, created_id};
use crate::support::{App, TestResult};

fn content_type(headers: &HeaderMap) -> &str {
    headers
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
}

#[sqlx::test]
async fn errors_are_problem_documents(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let missing = admin.get("/api/pages?path=nowhere").await?;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
    assert_eq!(content_type(&missing.headers), "application/problem+json");
    assert_eq!(missing.body["type"], "urn:openmemory:problem:not_found");
    assert_eq!(missing.body["title"], "Not found");
    assert_eq!(missing.body["status"], 404);

    let unknown = admin.get("/api/nothing-here").await?;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);
    assert_eq!(content_type(&unknown.headers), "application/problem+json");

    let anonymous = app.plain_get("/api/tree").await?;
    assert_eq!(anonymous.status, StatusCode::UNAUTHORIZED);
    assert_eq!(content_type(&anonymous.headers), "application/problem+json");
    Ok(())
}

#[sqlx::test]
async fn grammar_errors_carry_their_details_as_members(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let reply = create(&admin, "bad", "---\nstatus: active\n").await?;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(reply.body["type"], "urn:openmemory:problem:grammar");
    assert!(reply.body["line"].is_number());
    assert!(reply.body["reason"].is_string());
    Ok(())
}

#[sqlx::test]
async fn a_stale_write_returns_the_current_page(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let id = created_id(&admin, "a", PAGE).await?;
    let reply = admin
        .exchange(
            Method::PUT,
            &format!("/api/pages/{id}"),
            &[("if-match", "\"9\"")],
            Some(json!({"content": "x\n"})),
        )
        .await?;
    assert_eq!(reply.status, StatusCode::PRECONDITION_FAILED);
    assert_eq!(content_type(&reply.headers), "application/problem+json");
    assert_eq!(reply.body["current"]["content"], PAGE);
    assert_eq!(reply.headers["etag"], "\"1\"");
    Ok(())
}

#[sqlx::test]
async fn folders_list_one_level_with_a_cursor(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    for path in ["notes/a", "notes/b", "notes/c", "notes/deep/d"] {
        created_id(&admin, path, "x\n").await?;
    }
    let top = admin.get("/api/folder").await?;
    assert_eq!(top.status, StatusCode::OK);
    assert_eq!(top.body["path"], "/");
    assert_eq!(top.body["entries"][0]["name"], "notes");
    assert_eq!(top.body["entries"][0]["kind"], "folder");

    let first = admin.get("/api/folder?path=notes&limit=2").await?;
    assert_eq!(first.body["entries"].as_array().map(Vec::len), Some(2));
    let cursor = first.body["next"].as_str().ok_or("no cursor")?;
    let rest = admin
        .get(&format!("/api/folder?path=notes&limit=2&cursor={cursor}"))
        .await?;
    assert_eq!(rest.body["entries"].as_array().map(Vec::len), Some(2));
    assert_eq!(rest.body["entries"][1]["name"], "deep");
    assert!(rest.body["next"].is_null());

    let missing = admin.get("/api/folder?path=nowhere").await?;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
    Ok(())
}

#[sqlx::test]
async fn file_text_is_served_by_page(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let upload = admin
        .raw(
            Method::POST,
            "/api/files?path=docs/a.txt",
            &[("content-type", "text/plain")],
            b"hello text".to_vec(),
        )
        .await?;
    let created: Value = serde_json::from_slice(&upload.body)?;
    let id = created["id"].as_i64().ok_or("no id")?;
    let reply = admin.get(&format!("/api/files/{id}/text")).await?;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body["page"], 1);
    assert_eq!(reply.body["text"], "hello text");
    let beyond = admin.get(&format!("/api/files/{id}/text?page=9")).await?;
    assert_eq!(beyond.status, StatusCode::NOT_FOUND);
    assert_eq!(beyond.body["type"], "urn:openmemory:problem:page_not_found");
    Ok(())
}

#[sqlx::test]
async fn changes_page_through_a_cursor(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let member = app.member(&admin, "member").await?;
    for path in ["a", "b", "c"] {
        created_id(&member, path, "x\n").await?;
    }
    let first = admin.get("/api/changes?since=0&limit=2").await?;
    let cursor = first.body["next"].as_str().ok_or("no cursor")?.to_owned();
    let rest = admin.get(&format!("/api/changes?cursor={cursor}")).await?;
    let versions: usize = rest.body["groups"].as_array().map_or(0, |groups| {
        groups
            .iter()
            .map(|group| group["versions"].as_array().map_or(0, Vec::len))
            .sum()
    });
    assert_eq!(versions, 1);
    assert!(rest.body["next"].is_null());
    Ok(())
}
