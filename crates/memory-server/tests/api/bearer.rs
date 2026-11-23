use axum::http::{Method, StatusCode};
use serde_json::{Value, json};
use sqlx::PgPool;
use url::Url;

use crate::mcp::harness::grant_for;
use crate::oauth::{
    Grant, REDIRECT, RESOURCE, VERIFIER, approve, authorize_path, grant, location, param,
};
use crate::pages::{PAGE, create};
use crate::support::{App, Browser, Reply, TestResult};

const API_RESOURCE: &str = "http://localhost:8080/api";
const CLI: &str = "memory-cli";

async fn cli_consent(app: &App, redirect: &str) -> TestResult<String> {
    let reply = app
        .plain_get(&authorize_path(
            CLI,
            redirect,
            &[("resource", API_RESOURCE)],
        ))
        .await?;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{:?}", reply.body);
    param(&location(&reply)?, "consent").ok_or("no consent".into())
}

async fn api_grant(app: &App, user: &mut Browser, extra: Value) -> TestResult<Grant> {
    let reference = cli_consent(app, REDIRECT).await?;
    let approved = approve(user, &reference, extra).await?;
    assert_eq!(approved.status, StatusCode::OK, "{:?}", approved.body);
    let target = Url::parse(
        approved.body["redirect_uri"]
            .as_str()
            .ok_or("no redirect")?,
    )?;
    let code = param(&target, "code").ok_or("no code")?;
    grant(&exchange_api(app, &code).await?)
}

async fn exchange_api(app: &App, code: &str) -> TestResult<Reply> {
    app.form(
        "/oauth/token",
        &[
            ("grant_type", "authorization_code"),
            ("client_id", CLI),
            ("code", code),
            ("redirect_uri", REDIRECT),
            ("code_verifier", VERIFIER),
            ("resource", API_RESOURCE),
        ],
    )
    .await
}

async fn personal_token(admin: &Browser, name: &str, days: u32) -> TestResult<Reply> {
    let body = admin
        .confirmed(
            "create_token",
            json!({"name": name, "expires_in_days": days}),
        )
        .await?;
    admin.post("/api/tokens", body).await
}

fn challenge(reply: &Reply) -> &str {
    reply
        .headers
        .get("www-authenticate")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
}

async fn get(app: &App, path: &str, token: &str) -> TestResult<Reply> {
    app.bearer(Method::GET, path, token, &[], None).await
}

#[sqlx::test]
async fn each_resource_publishes_its_own_metadata(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let api = app
        .plain_get("/.well-known/oauth-protected-resource/api")
        .await?;
    assert_eq!(api.body["resource"], API_RESOURCE);
    let mcp = app
        .plain_get("/.well-known/oauth-protected-resource/mcp")
        .await?;
    assert_eq!(mcp.body["resource"], RESOURCE);
    let anonymous = app.plain_get("/api/tree").await?;
    assert_eq!(anonymous.status, StatusCode::UNAUTHORIZED);
    assert_eq!(
        challenge(&anonymous),
        r#"Bearer resource_metadata="http://localhost:8080/.well-known/oauth-protected-resource/api""#
    );
    Ok(())
}

#[sqlx::test]
async fn a_token_for_one_resource_is_rejected_by_the_other(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    let api = api_grant(&app, &mut admin, json!({"name": "cli"})).await?;
    let mcp = grant_for(&app, &mut admin, "agent", &["*"], true).await?;

    assert_eq!(
        get(&app, "/api/tree", &api.access).await?.status,
        StatusCode::OK
    );
    let wrong_api = get(&app, "/api/tree", &mcp.access).await?;
    assert_eq!(wrong_api.status, StatusCode::UNAUTHORIZED);
    assert!(challenge(&wrong_api).contains(r#"error="invalid_token""#));
    assert!(challenge(&wrong_api).contains("oauth-protected-resource/api"));

    let wrong_mcp = app
        .bearer(Method::POST, "/mcp", &api.access, &[], Some(json!({})))
        .await?;
    assert_eq!(wrong_mcp.status, StatusCode::UNAUTHORIZED);
    assert!(challenge(&wrong_mcp).contains(r#"error="invalid_token""#));
    assert!(challenge(&wrong_mcp).contains("oauth-protected-resource/mcp"));

    let refreshed = crate::oauth::refresh(&app, CLI, &api.refresh).await?;
    assert_eq!(
        get(&app, "/api/tree", &grant(&refreshed)?.access)
            .await?
            .status,
        StatusCode::OK
    );
    Ok(())
}

#[sqlx::test]
async fn the_token_endpoint_binds_the_requested_resource_to_the_grant(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    let reference = cli_consent(&app, REDIRECT).await?;
    let approved = approve(&mut admin, &reference, json!({})).await?;
    let target = Url::parse(
        approved.body["redirect_uri"]
            .as_str()
            .ok_or("no redirect")?,
    )?;
    let code = param(&target, "code").ok_or("no code")?;
    let reply = app
        .form(
            "/oauth/token",
            &[
                ("grant_type", "authorization_code"),
                ("client_id", CLI),
                ("code", &code),
                ("redirect_uri", REDIRECT),
                ("code_verifier", VERIFIER),
                ("resource", RESOURCE),
            ],
        )
        .await?;
    assert_eq!(reply.body["error"], "invalid_grant");
    Ok(())
}

#[sqlx::test]
async fn bearer_requests_get_the_connection_scope_without_cookie_checks(
    pool: PgPool,
) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    assert_eq!(
        create(&admin, "tax/a", PAGE).await?.status,
        StatusCode::CREATED
    );
    assert_eq!(
        create(&admin, "other/b", PAGE).await?.status,
        StatusCode::CREATED
    );
    let reader = api_grant(
        &app,
        &mut admin,
        json!({"name": "reader", "scopes": ["tax"], "can_write": false}),
    )
    .await?;

    let inside = get(&app, "/api/pages?path=tax/a", &reader.access).await?;
    assert_eq!(inside.status, StatusCode::OK, "{:?}", inside.body);
    let outside = get(&app, "/api/pages?path=other/b", &reader.access).await?;
    assert_eq!(outside.status, StatusCode::NOT_FOUND);
    let write = app
        .bearer(
            Method::PUT,
            "/api/pages?path=tax/c",
            &reader.access,
            &[("if-none-match", "*")],
            Some(json!({"content": PAGE})),
        )
        .await?;
    assert_eq!(write.status, StatusCode::FORBIDDEN);

    let writer = api_grant(
        &app,
        &mut admin,
        json!({"name": "writer", "scopes": ["tax"], "can_write": true}),
    )
    .await?;
    let created = app
        .bearer(
            Method::PUT,
            "/api/pages?path=tax/c",
            &writer.access,
            &[("if-none-match", "*")],
            Some(json!({"content": PAGE})),
        )
        .await?;
    assert_eq!(created.status, StatusCode::CREATED, "{:?}", created.body);
    let escaped = app
        .bearer(
            Method::PUT,
            "/api/pages?path=other/c",
            &writer.access,
            &[("if-none-match", "*")],
            Some(json!({"content": PAGE})),
        )
        .await?;
    assert_eq!(escaped.status, StatusCode::FORBIDDEN);
    Ok(())
}

#[sqlx::test]
async fn personal_tokens_are_shown_once_and_work_until_expiry_or_revocation(
    pool: PgPool,
) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    for days in [0, 366] {
        let bad = personal_token(&admin, "ci", days).await?;
        assert_eq!(bad.status, StatusCode::BAD_REQUEST, "{days}");
    }
    let created = personal_token(&admin, "ci", 30).await?;
    assert_eq!(created.status, StatusCode::CREATED, "{:?}", created.body);
    let token = created.body["token"].as_str().ok_or("no token")?.to_owned();
    assert!(token.starts_with("mpat_"));
    assert_eq!(created.body["name"], "ci");
    let id = created.body["id"].as_i64().ok_or("no id")?;

    let listed = admin.get("/api/connections").await?;
    assert!(!listed.body.to_string().contains(&token));
    assert_eq!(listed.body[0]["kind"], "token");
    assert_eq!(listed.body[0]["audience"], "api");
    assert!(listed.body[0]["expires_at"].is_string());
    assert!(listed.body[0]["last_used_at"].is_null());

    assert_eq!(get(&app, "/api/tree", &token).await?.status, StatusCode::OK);
    let used = admin.get("/api/connections").await?;
    assert!(used.body[0]["last_used_at"].is_string());
    assert_eq!(
        app.bearer(Method::POST, "/mcp", &token, &[], Some(json!({})))
            .await?
            .status,
        StatusCode::UNAUTHORIZED
    );
    let refresh = app
        .form(
            "/oauth/token",
            &[
                ("grant_type", "refresh_token"),
                ("client_id", CLI),
                ("refresh_token", &token),
            ],
        )
        .await?;
    assert_eq!(refresh.body["error"], "invalid_grant");

    sqlx::query("UPDATE connections SET expires_at = now() - interval '1 second'")
        .execute(&app.pool)
        .await?;
    assert_eq!(
        get(&app, "/api/tree", &token).await?.status,
        StatusCode::UNAUTHORIZED
    );

    let second = personal_token(&admin, "deploy", 1).await?;
    let second_token = second.body["token"].as_str().ok_or("no token")?.to_owned();
    assert_eq!(
        get(&app, "/api/tree", &second_token).await?.status,
        StatusCode::OK
    );
    let second_id = second.body["id"].as_i64().ok_or("no id")?;
    assert_ne!(second_id, id);
    assert_eq!(
        admin
            .delete(&format!("/api/connections/{second_id}"))
            .await?
            .status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        get(&app, "/api/tree", &second_token).await?.status,
        StatusCode::UNAUTHORIZED
    );
    Ok(())
}

#[sqlx::test]
async fn admin_routes_never_accept_a_bearer_token(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    let api = api_grant(&app, &mut admin, json!({"name": "cli"})).await?;
    let personal = personal_token(&admin, "ci", 30).await?;
    let personal = personal.body["token"]
        .as_str()
        .ok_or("no token")?
        .to_owned();
    for token in [api.access.as_str(), personal.as_str()] {
        for (method, path) in [
            (Method::GET, "/api/users"),
            (Method::GET, "/api/invites"),
            (Method::POST, "/api/invites"),
            (Method::GET, "/api/system"),
            (Method::POST, "/api/system/export"),
            (Method::GET, "/api/connections"),
            (Method::POST, "/api/tokens"),
            (Method::POST, "/api/auth/confirm"),
            (Method::POST, "/api/purge/challenge"),
            (Method::GET, "/api/auth/me"),
        ] {
            let reply = app
                .bearer(method.clone(), path, token, &[], Some(json!({})))
                .await?;
            assert_eq!(reply.status, StatusCode::FORBIDDEN, "{method} {path}");
            assert_eq!(reply.body["error"], "insufficient_scope", "{method} {path}");
            assert_eq!(challenge(&reply), r#"Bearer error="insufficient_scope""#);
        }
    }
    Ok(())
}

#[sqlx::test]
async fn the_official_cli_connects_from_any_loopback_port_only(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    for redirect in [
        "http://127.0.0.1:51234/callback",
        "http://[::1]:51234/callback",
    ] {
        let reference = cli_consent(&app, redirect).await?;
        let view = admin
            .get(&format!("/api/oauth/consents/{reference}"))
            .await?;
        assert_eq!(view.status, StatusCode::OK, "{:?}", view.body);
        assert_eq!(view.body["client"]["official"], true);
        assert_eq!(view.body["client"]["unverified"], false);
        assert_eq!(view.body["audience"], "api");
    }
    for redirect in [
        "https://evil.example/callback",
        "http://127.0.0.1:51234/other",
        "http://localhost:51234/callback",
    ] {
        let reply = app
            .plain_get(&authorize_path(
                CLI,
                redirect,
                &[("resource", API_RESOURCE)],
            ))
            .await?;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{redirect}");
    }
    let other = app
        .plain_get(&authorize_path(CLI, REDIRECT, &[("resource", "")]))
        .await?;
    assert_eq!(
        param(&location(&other)?, "error").as_deref(),
        Some("invalid_target")
    );
    Ok(())
}

#[sqlx::test]
async fn the_device_flow_for_the_cli_issues_an_api_token(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    let missing = app.form("/oauth/device", &[("client_id", CLI)]).await?;
    assert_eq!(missing.body["error"], "invalid_target");
    let start = app
        .form(
            "/oauth/device",
            &[("client_id", CLI), ("resource", API_RESOURCE)],
        )
        .await?;
    assert_eq!(start.status, StatusCode::OK, "{:?}", start.body);
    let user_code = start.body["user_code"].as_str().ok_or("no user code")?;
    let device_code = start.body["device_code"].as_str().ok_or("no device code")?;
    let view = admin
        .get(&format!("/api/oauth/consents/{user_code}"))
        .await?;
    assert_eq!(view.body["audience"], "api");
    assert_eq!(view.body["client"]["official"], true);
    let approved = approve(&mut admin, user_code, json!({"name": "laptop"})).await?;
    assert_eq!(approved.status, StatusCode::OK, "{:?}", approved.body);
    let reply = app
        .form(
            "/oauth/token",
            &[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("client_id", CLI),
                ("device_code", device_code),
                ("resource", API_RESOURCE),
            ],
        )
        .await?;
    let granted = grant(&reply)?;
    assert_eq!(
        get(&app, "/api/tree", &granted.access).await?.status,
        StatusCode::OK
    );
    Ok(())
}
