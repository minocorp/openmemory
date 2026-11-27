use axum::http::{Method, StatusCode};
use serde_json::{Value, json};
use sqlx::PgPool;

use crate::pages::{PAGE, created_id};
use crate::support::{App, Browser, Reply, TestResult};

async fn assertion(browser: &mut Browser) -> TestResult<Value> {
    let challenge = browser.post("/api/purge/challenge", json!({})).await?;
    assert_eq!(challenge.status, StatusCode::OK, "{:?}", challenge.body);
    let credential = browser.assert_credential(&challenge.body["options"])?;
    Ok(json!({"challenge": challenge.body["challenge"], "credential": credential}))
}

async fn purge(browser: &mut Browser, path: &str) -> TestResult<Reply> {
    let body = assertion(browser).await?;
    browser.post(path, body).await
}

async fn count(pool: &PgPool, sql: &'static str, id: i64) -> TestResult<i64> {
    Ok(sqlx::query_scalar(sql).bind(id).fetch_one(pool).await?)
}

async fn upload(browser: &Browser, path: &str, bytes: &[u8]) -> TestResult<i64> {
    let reply = browser
        .raw(
            Method::POST,
            &format!("/api/files?path={path}"),
            &[("content-type", "text/plain")],
            bytes.to_vec(),
        )
        .await?;
    assert_eq!(reply.status, StatusCode::CREATED);
    Ok(serde_json::from_slice::<Value>(&reply.body)?["id"]
        .as_i64()
        .ok_or("no id")?)
}

#[sqlx::test]
async fn purging_a_page_removes_every_version_and_leaves_a_tombstone(pool: PgPool) -> TestResult {
    let app = App::new(pool.clone());
    let (mut admin, _) = app.admin().await?;
    let id = created_id(&admin, "clients/acme", PAGE).await?;
    let edit = admin
        .exchange(
            Method::PATCH,
            &format!("/api/pages/{id}"),
            &[("if-match", "\"1\"")],
            Some(json!({"replace": {"old": "Partner", "new": "Vendor"}})),
        )
        .await?;
    assert_eq!(edit.status, StatusCode::OK);

    let purged = purge(&mut admin, &format!("/api/pages/{id}/purge")).await?;
    assert_eq!(purged.status, StatusCode::NO_CONTENT, "{:?}", purged.body);

    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM page_versions WHERE page_id = $1",
            id
        )
        .await?,
        0
    );
    assert_eq!(
        count(&pool, "SELECT count(*) FROM pages WHERE id = $1", id).await?,
        0
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM purges WHERE kind = 'page' AND subject_id = $1",
            id
        )
        .await?,
        1
    );
    let leftovers: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM purges WHERE path = 'clients/acme' AND kind = 'page'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(leftovers, 1, "the tombstone keeps the path but no content");

    assert_eq!(
        admin.get(&format!("/api/pages/{id}")).await?.status,
        StatusCode::GONE
    );
    assert_eq!(
        admin
            .get(&format!("/api/pages/{id}/versions"))
            .await?
            .status,
        StatusCode::GONE
    );
    assert_eq!(
        admin.get("/api/pages?path=clients/acme").await?.status,
        StatusCode::NOT_FOUND
    );
    assert!(
        admin.get("/api/search?q=Vendor").await?.body["hits"]
            .as_array()
            .is_some_and(Vec::is_empty)
    );
    created_id(&admin, "clients/acme", "again\n").await?;
    Ok(())
}

#[sqlx::test]
async fn purging_a_file_removes_the_row_and_its_text_index(pool: PgPool) -> TestResult {
    let app = App::new(pool.clone());
    let (mut admin, _) = app.admin().await?;
    let id = upload(&admin, "terms.txt", b"net 30 days\n").await?;
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM file_pages WHERE file_id = $1",
            id
        )
        .await?,
        1
    );
    let purged = purge(&mut admin, &format!("/api/files/{id}/purge")).await?;
    assert_eq!(purged.status, StatusCode::NO_CONTENT, "{:?}", purged.body);
    assert_eq!(
        count(&pool, "SELECT count(*) FROM files WHERE id = $1", id).await?,
        0
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM file_pages WHERE file_id = $1",
            id
        )
        .await?,
        0
    );
    assert_eq!(
        count(
            &pool,
            "SELECT count(*) FROM purges WHERE kind = 'file' AND subject_id = $1",
            id
        )
        .await?,
        1
    );
    assert_eq!(
        admin.get(&format!("/api/files/{id}")).await?.status,
        StatusCode::GONE
    );
    assert_eq!(
        admin.get(&format!("/api/files/{id}/content")).await?.status,
        StatusCode::GONE
    );
    Ok(())
}

#[sqlx::test]
async fn purging_needs_an_admin_and_a_fresh_purge_assertion(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    let member = app.member(&admin, "member").await?;
    let id = created_id(&admin, "a", "one\n").await?;
    let url = format!("/api/pages/{id}/purge");

    assert_eq!(
        member.post("/api/purge/challenge", json!({})).await?.status,
        StatusCode::FORBIDDEN
    );
    let body = assertion(&mut admin).await?;
    assert_eq!(
        member.post(&url, body).await?.status,
        StatusCode::FORBIDDEN,
        "members cannot purge even with an admin's assertion"
    );

    let login = admin
        .post("/api/auth/login/start", json!({"username": "admin"}))
        .await?;
    let credential = admin.assert_credential(&login.body["options"])?;
    let wrong_purpose = admin
        .post(
            &url,
            json!({"challenge": login.body["challenge"], "credential": credential}),
        )
        .await?;
    assert_eq!(wrong_purpose.status, StatusCode::BAD_REQUEST);

    let own = assertion(&mut admin).await?;
    let anonymous = app.browser();
    assert_eq!(
        anonymous.post(&url, own).await?.status,
        StatusCode::UNAUTHORIZED
    );

    let again = assertion(&mut admin).await?;
    assert_eq!(
        admin.post(&url, again.clone()).await?.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        admin
            .post(&format!("/api/pages/{id}/purge"), again)
            .await?
            .status,
        StatusCode::BAD_REQUEST,
        "an assertion works once"
    );
    assert_eq!(
        admin.get(&format!("/api/pages/{id}")).await?.status,
        StatusCode::GONE
    );
    Ok(())
}

#[sqlx::test]
async fn purging_rejects_the_start_page_and_unknown_ids(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    let start = created_id(&admin, "start", "Rules.\n").await?;
    let reserved = purge(&mut admin, &format!("/api/pages/{start}/purge")).await?;
    assert_eq!(reserved.status, StatusCode::BAD_REQUEST);
    assert_eq!(
        reserved.body["type"],
        "urn:openmemory:problem:reserved_path"
    );
    assert_eq!(
        admin.get(&format!("/api/pages/{start}")).await?.status,
        StatusCode::OK
    );

    assert_eq!(
        purge(&mut admin, "/api/pages/999/purge").await?.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        purge(&mut admin, "/api/files/999/purge").await?.status,
        StatusCode::NOT_FOUND
    );
    Ok(())
}
