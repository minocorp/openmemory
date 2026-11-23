use axum::http::StatusCode;
use serde_json::json;
use sqlx::PgPool;

use crate::oauth::{REDIRECT, VERIFIER, code_for, connected, exchange, refresh, register};
use crate::support::{App, TestResult};

#[sqlx::test]
async fn people_see_their_own_connections_and_admins_see_all(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    let mut member = app.member(&admin, "member").await?;
    connected(&app, &mut admin).await?;
    connected(&app, &mut member).await?;

    let mine = member.get("/api/connections").await?;
    assert_eq!(mine.body.as_array().map(Vec::len), Some(1));
    assert_eq!(mine.body[0]["owner"], "member");
    let all = admin.get("/api/connections").await?;
    assert_eq!(all.body.as_array().map(Vec::len), Some(2));
    assert_eq!(all.body[0]["owner"], "admin");
    assert_eq!(all.body[1]["owner"], "member");
    Ok(())
}

#[sqlx::test]
async fn owners_rename_and_rescope_their_connections(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    connected(&app, &mut admin).await?;
    let id = admin.get("/api/connections").await?.body[0]["id"]
        .as_i64()
        .ok_or("no id")?;

    let changed = admin
        .patch(
            &format!("/api/connections/{id}"),
            json!({"name": "memory-bot", "scopes": ["tax"], "can_write": false}),
        )
        .await?;
    assert_eq!(changed.status, StatusCode::OK, "{:?}", changed.body);
    assert_eq!(changed.body["name"], "memory-bot");
    assert_eq!(changed.body["scopes"], json!(["tax"]));
    assert_eq!(changed.body["can_write"], false);

    let invalid = admin
        .patch(
            &format!("/api/connections/{id}"),
            json!({"scopes": ["a//b"]}),
        )
        .await?;
    assert_eq!(invalid.status, StatusCode::BAD_REQUEST);
    let kept = admin.get("/api/connections").await?;
    assert_eq!(kept.body[0]["scopes"], json!(["tax"]));
    Ok(())
}

#[sqlx::test]
async fn renaming_into_an_existing_name_conflicts(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    connected(&app, &mut admin).await?;
    connected(&app, &mut admin).await?;
    let list = admin.get("/api/connections").await?;
    let first = list.body[0]["name"].as_str().ok_or("no name")?.to_owned();
    let second_id = list.body[1]["id"].as_i64().ok_or("no id")?;
    let clash = admin
        .patch(
            &format!("/api/connections/{second_id}"),
            json!({"name": first}),
        )
        .await?;
    assert_eq!(clash.status, StatusCode::CONFLICT);
    Ok(())
}

#[sqlx::test]
async fn only_owners_and_admins_manage_a_connection(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    let member = app.member(&admin, "member").await?;
    connected(&app, &mut admin).await?;
    let id = admin.get("/api/connections").await?.body[0]["id"]
        .as_i64()
        .ok_or("no id")?;

    let path = format!("/api/connections/{id}");
    assert_eq!(
        member.patch(&path, json!({"name": "mine"})).await?.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(member.delete(&path).await?.status, StatusCode::NOT_FOUND);
    assert_eq!(
        admin
            .get("/api/connections")
            .await?
            .body
            .as_array()
            .map(Vec::len),
        Some(1)
    );

    let mut other = app.member(&admin, "other").await?;
    connected(&app, &mut other).await?;
    let others_id = member_connection_id(&other).await?;
    let removed = admin
        .delete(&format!("/api/connections/{others_id}"))
        .await?;
    assert_eq!(removed.status, StatusCode::NO_CONTENT);
    assert_eq!(other.get("/api/connections").await?.body, json!([]));
    Ok(())
}

async fn member_connection_id(browser: &crate::support::Browser) -> TestResult<i64> {
    Ok(browser.get("/api/connections").await?.body[0]["id"]
        .as_i64()
        .ok_or("no id")?)
}

#[sqlx::test]
async fn a_revoked_connection_is_gone_from_the_list_and_cannot_be_edited(
    pool: PgPool,
) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    connected(&app, &mut admin).await?;
    let id = member_connection_id(&admin).await?;
    let path = format!("/api/connections/{id}");
    assert_eq!(admin.delete(&path).await?.status, StatusCode::NO_CONTENT);
    assert_eq!(admin.get("/api/connections").await?.body, json!([]));
    assert_eq!(
        admin.patch(&path, json!({"name": "x"})).await?.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(admin.delete(&path).await?.status, StatusCode::NOT_FOUND);
    Ok(())
}

#[sqlx::test]
async fn a_revoked_name_can_be_reused(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    connected(&app, &mut admin).await?;
    let id = member_connection_id(&admin).await?;
    admin.delete(&format!("/api/connections/{id}")).await?;
    connected(&app, &mut admin).await?;
    assert_eq!(
        admin.get("/api/connections").await?.body[0]["name"],
        "claude-code"
    );
    Ok(())
}

#[sqlx::test]
async fn connection_changes_need_a_session_and_a_matching_origin(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    connected(&app, &mut admin).await?;
    let id = member_connection_id(&admin).await?;
    let anonymous = app.browser();
    assert_eq!(
        anonymous
            .delete(&format!("/api/connections/{id}"))
            .await?
            .status,
        StatusCode::UNAUTHORIZED
    );
    let forged = axum::http::Request::delete(format!("/api/connections/{id}"))
        .header("cookie", admin.clone_cookie())
        .header("origin", "https://evil.example")
        .header("content-type", "application/json")
        .body(axum::body::Body::empty())?;
    assert_eq!(admin.send(forged).await?.status, StatusCode::FORBIDDEN);
    assert_eq!(
        admin
            .get("/api/connections")
            .await?
            .body
            .as_array()
            .map(Vec::len),
        Some(1)
    );
    Ok(())
}

#[sqlx::test]
async fn recovery_revokes_every_connection(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (mut admin, code) = app.admin().await?;
    let (client_id, granted) = connected(&app, &mut admin).await?;
    let waiting_client = register(&app, REDIRECT).await?;
    let pending_code = code_for(&app, &mut admin, &waiting_client).await?;

    let mut device = app.browser();
    let start = device
        .post(
            "/api/auth/recovery/start",
            json!({"username": "admin", "code": code}),
        )
        .await?;
    let credential = device.register_credential(&start.body["options"])?;
    let finish = device
        .post(
            "/api/auth/recovery/finish",
            json!({"challenge": start.body["challenge"], "credential": credential}),
        )
        .await?;
    assert_eq!(finish.status, StatusCode::OK);
    device.adopt_cookie(&finish);

    assert_eq!(device.get("/api/connections").await?.body, json!([]));
    let reply = refresh(&app, &client_id, &granted.refresh).await?;
    assert_eq!(reply.body["error"], "invalid_grant");
    let late = exchange(&app, &waiting_client, &pending_code, VERIFIER).await?;
    assert_eq!(late.body["error"], "invalid_grant");
    Ok(())
}
