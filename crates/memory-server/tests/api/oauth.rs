use axum::http::StatusCode;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use url::Url;

use crate::support::{App, Browser, Reply, TestResult};

pub(crate) const RESOURCE: &str = "http://localhost:8080/mcp";
pub(crate) const REDIRECT: &str = "http://127.0.0.1:3000/callback";
pub(crate) const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";

pub(crate) struct Grant {
    pub(crate) access: String,
    pub(crate) refresh: String,
}

pub(crate) fn challenge() -> String {
    URL_SAFE_NO_PAD.encode(Sha256::digest(VERIFIER.as_bytes()))
}

pub(crate) async fn register(app: &App, redirect_uri: &str) -> TestResult<String> {
    let reply = app
        .router_post_json(
            "/oauth/register",
            json!({"client_name": "Claude Code", "redirect_uris": [redirect_uri]}),
        )
        .await?;
    assert_eq!(reply.status, StatusCode::CREATED, "{:?}", reply.body);
    Ok(reply.body["client_id"]
        .as_str()
        .ok_or("no client id")?
        .to_owned())
}

pub(crate) fn authorize_path(
    client_id: &str,
    redirect_uri: &str,
    extra: &[(&str, &str)],
) -> String {
    let challenge = challenge();
    let mut pairs = vec![
        ("response_type", "code"),
        ("client_id", client_id),
        ("redirect_uri", redirect_uri),
        ("code_challenge", challenge.as_str()),
        ("code_challenge_method", "S256"),
        ("state", "xyz"),
        ("resource", RESOURCE),
    ];
    pairs.retain(|(key, _)| !extra.iter().any(|(name, _)| name == key));
    pairs.extend(extra.iter().filter(|(_, value)| !value.is_empty()));
    let query = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(pairs)
        .finish();
    format!("/oauth/authorize?{query}")
}

pub(crate) fn location(reply: &Reply) -> TestResult<Url> {
    let value = reply
        .headers
        .get("location")
        .ok_or("no location")?
        .to_str()?;
    Ok(Url::parse("http://localhost:8080")?.join(value)?)
}

pub(crate) fn param(url: &Url, name: &str) -> Option<String> {
    url.query_pairs()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value.into_owned())
}

pub(crate) async fn consent(app: &App, client_id: &str) -> TestResult<String> {
    let reply = app
        .plain_get(&authorize_path(client_id, REDIRECT, &[]))
        .await?;
    assert_eq!(reply.status, StatusCode::SEE_OTHER, "{:?}", reply.body);
    let target = location(&reply)?;
    assert_eq!(target.path(), "/approve");
    param(&target, "consent").ok_or("no consent".into())
}

pub(crate) async fn approve(
    user: &mut Browser,
    reference: &str,
    extra: Value,
) -> TestResult<Reply> {
    let challenge = user
        .post(
            &format!("/api/oauth/consents/{reference}/challenge"),
            json!({}),
        )
        .await?;
    assert_eq!(challenge.status, StatusCode::OK, "{:?}", challenge.body);
    let credential = user.assert_credential(&challenge.body["options"])?;
    let mut body = json!({"challenge": challenge.body["challenge"], "credential": credential});
    if let (Some(target), Some(source)) = (body.as_object_mut(), extra.as_object()) {
        target.extend(source.clone());
    }
    user.post(&format!("/api/oauth/consents/{reference}/approve"), body)
        .await
}

pub(crate) async fn code_for(app: &App, user: &mut Browser, client_id: &str) -> TestResult<String> {
    let reference = consent(app, client_id).await?;
    let approved = approve(user, &reference, json!({})).await?;
    assert_eq!(approved.status, StatusCode::OK, "{:?}", approved.body);
    let target = Url::parse(
        approved.body["redirect_uri"]
            .as_str()
            .ok_or("no redirect")?,
    )?;
    assert_eq!(param(&target, "state").as_deref(), Some("xyz"));
    assert_eq!(
        param(&target, "iss").as_deref(),
        Some("http://localhost:8080")
    );
    param(&target, "code").ok_or("no code".into())
}

pub(crate) async fn exchange(
    app: &App,
    client_id: &str,
    code: &str,
    verifier: &str,
) -> TestResult<Reply> {
    app.form(
        "/oauth/token",
        &[
            ("grant_type", "authorization_code"),
            ("client_id", client_id),
            ("code", code),
            ("redirect_uri", REDIRECT),
            ("code_verifier", verifier),
            ("resource", RESOURCE),
        ],
    )
    .await
}

pub(crate) async fn refresh(app: &App, client_id: &str, token: &str) -> TestResult<Reply> {
    app.form(
        "/oauth/token",
        &[
            ("grant_type", "refresh_token"),
            ("client_id", client_id),
            ("refresh_token", token),
        ],
    )
    .await
}

pub(crate) fn grant(reply: &Reply) -> TestResult<Grant> {
    assert_eq!(reply.status, StatusCode::OK, "{:?}", reply.body);
    Ok(Grant {
        access: reply.body["access_token"]
            .as_str()
            .ok_or("no access")?
            .to_owned(),
        refresh: reply.body["refresh_token"]
            .as_str()
            .ok_or("no refresh")?
            .to_owned(),
    })
}

pub(crate) async fn connected(app: &App, user: &mut Browser) -> TestResult<(String, Grant)> {
    let client_id = register(app, REDIRECT).await?;
    let code = code_for(app, user, &client_id).await?;
    let reply = exchange(app, &client_id, &code, VERIFIER).await?;
    Ok((client_id, grant(&reply)?))
}

#[sqlx::test]
async fn metadata_documents_describe_the_server(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let server = app
        .plain_get("/.well-known/oauth-authorization-server")
        .await?;
    assert_eq!(server.status, StatusCode::OK);
    assert_eq!(server.body["issuer"], "http://localhost:8080");
    assert_eq!(
        server.body["code_challenge_methods_supported"],
        json!(["S256"])
    );
    assert_eq!(server.body["client_id_metadata_document_supported"], true);
    assert_eq!(
        server.body["token_endpoint"],
        "http://localhost:8080/oauth/token"
    );
    let resource = app
        .plain_get("/.well-known/oauth-protected-resource/mcp")
        .await?;
    assert_eq!(resource.body["resource"], RESOURCE);
    assert_eq!(
        resource.body["authorization_servers"],
        json!(["http://localhost:8080"])
    );
    Ok(())
}

#[sqlx::test]
async fn registration_rejects_unsafe_redirects_and_secret_auth(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    for body in [
        json!({"redirect_uris": ["http://evil.example/cb"]}),
        json!({"redirect_uris": ["cursor://cb"]}),
        json!({"redirect_uris": []}),
        json!({}),
    ] {
        let reply = app.router_post_json("/oauth/register", body).await?;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST);
        assert_eq!(reply.body["error"], "invalid_redirect_uri");
    }
    let secret = app
        .router_post_json(
            "/oauth/register",
            json!({"redirect_uris": [REDIRECT], "token_endpoint_auth_method": "client_secret_basic"}),
        )
        .await?;
    assert_eq!(secret.body["error"], "invalid_client_metadata");
    Ok(())
}

#[sqlx::test]
async fn the_full_code_flow_creates_a_scoped_connection_and_tokens(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    let client_id = register(&app, REDIRECT).await?;
    let reference = consent(&app, &client_id).await?;

    let view = admin
        .get(&format!("/api/oauth/consents/{reference}"))
        .await?;
    assert_eq!(view.status, StatusCode::OK, "{:?}", view.body);
    assert_eq!(view.body["client"]["name"], "Claude Code");
    assert_eq!(view.body["proposed_name"], "claude-code");
    assert_eq!(view.body["redirect_host"], "127.0.0.1");
    assert_eq!(view.body["localhost_only"], true);
    assert_eq!(view.body["client"]["unverified"], true);
    assert_eq!(view.body["name_matches_redirect"], Value::Null);
    assert_eq!(view.body["default_scopes"], json!(["*"]));
    assert_eq!(view.body["default_can_write"], true);

    let approved = approve(
        &mut admin,
        &reference,
        json!({"name": "cc", "scopes": ["tax", "clients/acme"], "can_write": false}),
    )
    .await?;
    assert_eq!(approved.status, StatusCode::OK, "{:?}", approved.body);
    let target = Url::parse(approved.body["redirect_uri"].as_str().ok_or("no uri")?)?;
    let code = param(&target, "code").ok_or("no code")?;

    let reply = exchange(&app, &client_id, &code, VERIFIER).await?;
    let granted = grant(&reply)?;
    assert_eq!(reply.body["token_type"], "Bearer");
    assert_eq!(reply.headers["cache-control"], "no-store");
    assert_ne!(granted.access, granted.refresh);

    let list = admin.get("/api/connections").await?;
    assert_eq!(list.body[0]["name"], "cc");
    assert_eq!(list.body[0]["scopes"], json!(["tax", "clients/acme"]));
    assert_eq!(list.body[0]["can_write"], false);
    Ok(())
}

#[sqlx::test]
async fn approval_defaults_to_all_folders_with_writing(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    connected(&app, &mut admin).await?;
    let list = admin.get("/api/connections").await?;
    assert_eq!(list.body[0]["scopes"], json!(["*"]));
    assert_eq!(list.body[0]["can_write"], true);
    assert_eq!(list.body[0]["name"], "claude-code");
    Ok(())
}

#[sqlx::test]
async fn tokens_are_stored_hashed_and_codes_are_single_use(pool: PgPool) -> TestResult {
    let app = App::new(pool.clone());
    let (mut admin, _) = app.admin().await?;
    let client_id = register(&app, REDIRECT).await?;
    let code = code_for(&app, &mut admin, &client_id).await?;
    let granted = grant(&exchange(&app, &client_id, &code, VERIFIER).await?)?;
    let stored: Vec<Vec<u8>> = sqlx::query_scalar("SELECT access_token_hash FROM oauth_tokens")
        .fetch_all(&pool)
        .await?;
    assert_eq!(stored.len(), 1);
    assert_ne!(stored[0], granted.access.as_bytes());
    assert_eq!(stored[0].len(), 32);

    let replay = exchange(&app, &client_id, &code, VERIFIER).await?;
    assert_eq!(replay.body["error"], "invalid_grant");
    let after = refresh(&app, &client_id, &granted.refresh).await?;
    assert_eq!(
        after.body["error"], "invalid_grant",
        "replay revokes the connection"
    );
    Ok(())
}

#[sqlx::test]
async fn pkce_is_required_and_verified(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    let client_id = register(&app, REDIRECT).await?;

    for extra in [[("code_challenge", "")], [("code_challenge", "tooshort")]] {
        let reply = app
            .plain_get(&authorize_path(&client_id, REDIRECT, &extra))
            .await?;
        assert_eq!(reply.status, StatusCode::SEE_OTHER);
        let target = location(&reply)?;
        assert_eq!(target.host_str(), Some("127.0.0.1"));
        assert_eq!(param(&target, "error").as_deref(), Some("invalid_request"));
        assert_eq!(param(&target, "state").as_deref(), Some("xyz"));
    }
    let plain = app
        .plain_get(&authorize_path(
            &client_id,
            REDIRECT,
            &[("code_challenge_method", "plain")],
        ))
        .await?;
    assert_eq!(
        param(&location(&plain)?, "error").as_deref(),
        Some("invalid_request")
    );

    let code = code_for(&app, &mut admin, &client_id).await?;
    let wrong = exchange(&app, &client_id, &code, &"a".repeat(43)).await?;
    assert_eq!(wrong.status, StatusCode::BAD_REQUEST);
    assert_eq!(wrong.body["error"], "invalid_grant");
    let burned = exchange(&app, &client_id, &code, VERIFIER).await?;
    assert_eq!(
        burned.body["error"], "invalid_grant",
        "a failed attempt burns the code"
    );
    Ok(())
}

#[sqlx::test]
async fn a_missing_verifier_is_an_invalid_request(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let client_id = register(&app, REDIRECT).await?;
    let reply = app
        .form(
            "/oauth/token",
            &[
                ("grant_type", "authorization_code"),
                ("client_id", &client_id),
                ("code", "x"),
                ("redirect_uri", REDIRECT),
            ],
        )
        .await?;
    assert_eq!(reply.body["error"], "invalid_request");
    Ok(())
}

#[sqlx::test]
async fn redirect_uris_must_match_the_registration(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let client_id = register(&app, "https://app.example/callback").await?;
    for uri in [
        "https://app.example/callback/",
        "https://app.example/other",
        "https://evil.example/callback",
        "http://app.example/callback",
    ] {
        let reply = app.plain_get(&authorize_path(&client_id, uri, &[])).await?;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST, "{uri}");
        assert!(reply.headers.get("location").is_none());
        assert_eq!(reply.body["error"], "invalid_request");
    }
    let missing = app
        .plain_get(&authorize_path(&client_id, "", &[("redirect_uri", "")]))
        .await?;
    assert_eq!(missing.status, StatusCode::BAD_REQUEST);
    Ok(())
}

#[sqlx::test]
async fn loopback_redirects_accept_any_port(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let client_id = register(&app, REDIRECT).await?;
    let reply = app
        .plain_get(&authorize_path(
            &client_id,
            "http://127.0.0.1:61234/callback",
            &[],
        ))
        .await?;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert_eq!(location(&reply)?.path(), "/approve");
    let other_path = app
        .plain_get(&authorize_path(
            &client_id,
            "http://127.0.0.1:61234/elsewhere",
            &[],
        ))
        .await?;
    assert_eq!(other_path.status, StatusCode::BAD_REQUEST);
    Ok(())
}

#[sqlx::test]
async fn the_resource_must_be_this_servers_mcp_url(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let client_id = register(&app, REDIRECT).await?;
    for resource in [
        "https://other.example/mcp",
        "http://localhost:8080",
        "http://localhost:8080/mcp/x",
        "",
    ] {
        let reply = app
            .plain_get(&authorize_path(
                &client_id,
                REDIRECT,
                &[("resource", resource)],
            ))
            .await?;
        let error = param(&location(&reply)?, "error");
        assert_eq!(error.as_deref(), Some("invalid_target"), "{resource:?}");
    }
    let token = app
        .form(
            "/oauth/token",
            &[
                ("grant_type", "refresh_token"),
                ("client_id", &client_id),
                ("refresh_token", "x"),
                ("resource", "https://other.example/mcp"),
            ],
        )
        .await?;
    assert_eq!(token.body["error"], "invalid_target");
    Ok(())
}

#[sqlx::test]
async fn the_token_exchange_binds_client_and_redirect(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    let client_id = register(&app, REDIRECT).await?;
    let other = register(&app, REDIRECT).await?;

    let code = code_for(&app, &mut admin, &client_id).await?;
    let stolen = exchange(&app, &other, &code, VERIFIER).await?;
    assert_eq!(stolen.body["error"], "invalid_grant");

    let code = code_for(&app, &mut admin, &client_id).await?;
    let wrong_redirect = app
        .form(
            "/oauth/token",
            &[
                ("grant_type", "authorization_code"),
                ("client_id", &client_id),
                ("code", &code),
                ("redirect_uri", "http://127.0.0.1:3000/other"),
                ("code_verifier", VERIFIER),
            ],
        )
        .await?;
    assert_eq!(wrong_redirect.body["error"], "invalid_grant");

    let unknown = exchange(&app, "dcr-unknown", "x", VERIFIER).await?;
    assert_eq!(unknown.status, StatusCode::UNAUTHORIZED);
    assert_eq!(unknown.body["error"], "invalid_client");
    Ok(())
}

#[sqlx::test]
async fn expired_codes_and_consents_are_refused(pool: PgPool) -> TestResult {
    let app = App::new(pool.clone());
    let (mut admin, _) = app.admin().await?;
    let client_id = register(&app, REDIRECT).await?;
    let code = code_for(&app, &mut admin, &client_id).await?;
    sqlx::query("UPDATE oauth_codes SET created_at = now() - interval '2 hours', expires_at = now() - interval '1 hour'")
        .execute(&pool)
        .await?;
    let reply = exchange(&app, &client_id, &code, VERIFIER).await?;
    assert_eq!(reply.body["error"], "invalid_grant");

    let reference = consent(&app, &client_id).await?;
    sqlx::query("UPDATE oauth_consents SET created_at = now() - interval '2 hours', expires_at = now() - interval '1 hour'")
        .execute(&pool)
        .await?;
    let view = admin
        .get(&format!("/api/oauth/consents/{reference}"))
        .await?;
    assert_eq!(view.status, StatusCode::NOT_FOUND);
    Ok(())
}

#[sqlx::test]
async fn refresh_tokens_rotate_and_reuse_revokes_the_connection(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    let (client_id, first) = connected(&app, &mut admin).await?;

    let second = grant(&refresh(&app, &client_id, &first.refresh).await?)?;
    assert_ne!(second.refresh, first.refresh);
    assert_ne!(second.access, first.access);
    let third = grant(&refresh(&app, &client_id, &second.refresh).await?)?;

    let reuse = refresh(&app, &client_id, &first.refresh).await?;
    assert_eq!(reuse.status, StatusCode::BAD_REQUEST);
    assert_eq!(reuse.body["error"], "invalid_grant");
    let chain = refresh(&app, &client_id, &third.refresh).await?;
    assert_eq!(
        chain.body["error"], "invalid_grant",
        "the whole chain is revoked"
    );
    assert_eq!(admin.get("/api/connections").await?.body, json!([]));
    Ok(())
}

#[sqlx::test]
async fn refresh_tokens_belong_to_their_client_and_expire(pool: PgPool) -> TestResult {
    let app = App::new(pool.clone());
    let (mut admin, _) = app.admin().await?;
    let (client_id, granted) = connected(&app, &mut admin).await?;
    let other = register(&app, REDIRECT).await?;
    let stolen = refresh(&app, &other, &granted.refresh).await?;
    assert_eq!(stolen.body["error"], "invalid_grant");

    sqlx::query("UPDATE oauth_tokens SET created_at = now() - interval '40 days', access_expires_at = now() - interval '39 days', refresh_expires_at = now() - interval '10 days'")
        .execute(&pool)
        .await?;
    let expired = refresh(&app, &client_id, &granted.refresh).await?;
    assert_eq!(expired.body["error"], "invalid_grant");
    Ok(())
}

#[sqlx::test]
async fn revoking_a_connection_stops_refresh_immediately(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    let (client_id, granted) = connected(&app, &mut admin).await?;
    let list = admin.get("/api/connections").await?;
    let id = list.body[0]["id"].as_i64().ok_or("no id")?;
    let removed = admin.delete(&format!("/api/connections/{id}")).await?;
    assert_eq!(removed.status, StatusCode::NO_CONTENT);
    let reply = refresh(&app, &client_id, &granted.refresh).await?;
    assert_eq!(reply.body["error"], "invalid_grant");
    Ok(())
}

#[sqlx::test]
async fn revoking_another_clients_token_changes_nothing(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    let (client_id, granted) = connected(&app, &mut admin).await?;
    let other = register(&app, REDIRECT).await?;
    let foreign = app
        .form(
            "/oauth/revoke",
            &[("token", &granted.refresh), ("client_id", &other)],
        )
        .await?;
    assert_eq!(foreign.status, StatusCode::OK);
    grant(&refresh(&app, &client_id, &granted.refresh).await?)?;
    Ok(())
}

#[sqlx::test]
async fn revoking_the_refresh_token_ends_access(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    let (client_id, granted) = connected(&app, &mut admin).await?;
    let unknown = app
        .form(
            "/oauth/revoke",
            &[("token", "nonsense"), ("client_id", &client_id)],
        )
        .await?;
    assert_eq!(unknown.status, StatusCode::OK);
    let revoked = app
        .form(
            "/oauth/revoke",
            &[("token", &granted.refresh), ("client_id", &client_id)],
        )
        .await?;
    assert_eq!(revoked.status, StatusCode::OK);
    let reply = refresh(&app, &client_id, &granted.refresh).await?;
    assert_eq!(reply.body["error"], "invalid_grant");
    assert_eq!(admin.get("/api/connections").await?.body, json!([]));
    Ok(())
}

#[sqlx::test]
async fn approval_needs_a_session_and_a_fresh_approval_assertion(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let client_id = register(&app, REDIRECT).await?;
    let reference = consent(&app, &client_id).await?;

    let anonymous = app.browser();
    let reply = anonymous
        .get(&format!("/api/oauth/consents/{reference}"))
        .await?;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);

    let login = admin
        .post("/api/auth/login/start", json!({"username": "admin"}))
        .await?;
    let credential = admin.assert_credential(&login.body["options"])?;
    let misused = admin
        .post(
            &format!("/api/oauth/consents/{reference}/approve"),
            json!({"challenge": login.body["challenge"], "credential": credential}),
        )
        .await?;
    assert_eq!(
        misused.status,
        StatusCode::BAD_REQUEST,
        "login challenges cannot approve"
    );

    let challenge = admin
        .post(
            &format!("/api/oauth/consents/{reference}/challenge"),
            json!({}),
        )
        .await?;
    let credential = admin.assert_credential(&challenge.body["options"])?;
    let body = json!({"challenge": challenge.body["challenge"], "credential": credential});
    let ok = admin
        .post(
            &format!("/api/oauth/consents/{reference}/approve"),
            body.clone(),
        )
        .await?;
    assert_eq!(ok.status, StatusCode::OK);
    let replay = admin
        .post(&format!("/api/oauth/consents/{reference}/approve"), body)
        .await?;
    assert_eq!(
        replay.status,
        StatusCode::BAD_REQUEST,
        "an assertion is single use"
    );
    Ok(())
}

#[sqlx::test]
async fn another_users_assertion_cannot_approve(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let member = app.member(&admin, "member").await?;
    let client_id = register(&app, REDIRECT).await?;
    let reference = consent(&app, &client_id).await?;

    let challenge = member
        .post(
            &format!("/api/oauth/consents/{reference}/challenge"),
            json!({}),
        )
        .await?;
    let credential = member.assert_credential(&challenge.body["options"])?;
    let forged = admin
        .post(
            &format!("/api/oauth/consents/{reference}/approve"),
            json!({"challenge": challenge.body["challenge"], "credential": credential}),
        )
        .await?;
    assert_eq!(forged.status, StatusCode::UNAUTHORIZED);
    let list = admin.get("/api/connections").await?;
    assert_eq!(list.body, json!([]));
    Ok(())
}

#[sqlx::test]
async fn connection_names_are_unique_per_user(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    let client_id = register(&app, REDIRECT).await?;
    let first = consent(&app, &client_id).await?;
    let second = consent(&app, &client_id).await?;
    let ok = approve(&mut admin, &first, json!({"name": "agent"})).await?;
    assert_eq!(ok.status, StatusCode::OK);
    let clash = approve(&mut admin, &second, json!({"name": "agent"})).await?;
    assert_eq!(clash.status, StatusCode::CONFLICT);
    let view = admin.get(&format!("/api/oauth/consents/{second}")).await?;
    assert_eq!(view.body["proposed_name"], "claude-code");
    let list = admin.get("/api/connections").await?;
    assert_eq!(list.body.as_array().map(Vec::len), Some(1));
    Ok(())
}

#[sqlx::test]
async fn approval_validates_scopes_and_names(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    let client_id = register(&app, REDIRECT).await?;
    let reference = consent(&app, &client_id).await?;
    let bad_scope = approve(&mut admin, &reference, json!({"scopes": ["a//b"]})).await?;
    assert_eq!(bad_scope.status, StatusCode::BAD_REQUEST);
    let mixed = approve(&mut admin, &reference, json!({"scopes": ["*", "tax"]})).await?;
    assert_eq!(mixed.status, StatusCode::BAD_REQUEST);
    let empty = approve(&mut admin, &reference, json!({"scopes": []})).await?;
    assert_eq!(empty.status, StatusCode::BAD_REQUEST);
    Ok(())
}

#[sqlx::test]
async fn denying_redirects_with_access_denied(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let client_id = register(&app, REDIRECT).await?;
    let reference = consent(&app, &client_id).await?;
    let denied = admin
        .post(&format!("/api/oauth/consents/{reference}/deny"), json!({}))
        .await?;
    let target = Url::parse(denied.body["redirect_uri"].as_str().ok_or("no uri")?)?;
    assert_eq!(param(&target, "error").as_deref(), Some("access_denied"));
    assert_eq!(param(&target, "state").as_deref(), Some("xyz"));
    assert_eq!(admin.get("/api/connections").await?.body, json!([]));
    Ok(())
}

#[sqlx::test]
async fn private_and_loopback_client_metadata_urls_are_never_fetched(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    for client_id in [
        "https://127.0.0.1/client.json",
        "https://localhost/client.json",
        "https://[::1]/client.json",
        "https://169.254.169.254/latest/meta-data/client.json",
        "https://10.0.0.5/client.json",
        "https://192.168.1.1:8443/client.json",
        "https://[::ffff:127.0.0.1]/client.json",
        "http://app.example/client.json",
        "https://app.example/",
    ] {
        let reply = app
            .plain_get(&authorize_path(client_id, REDIRECT, &[]))
            .await?;
        assert_eq!(reply.status, StatusCode::UNAUTHORIZED, "{client_id}");
        assert_eq!(reply.body["error"], "invalid_client", "{client_id}");
    }
    let device = app
        .form(
            "/oauth/device",
            &[("client_id", "https://127.0.0.1/client.json")],
        )
        .await?;
    assert_eq!(device.status, StatusCode::UNAUTHORIZED);
    Ok(())
}

#[sqlx::test]
async fn the_token_endpoint_rejects_unknown_grants_and_client_secrets(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let client_id = register(&app, REDIRECT).await?;
    let grant = app
        .form(
            "/oauth/token",
            &[
                ("grant_type", "client_credentials"),
                ("client_id", &client_id),
            ],
        )
        .await?;
    assert_eq!(grant.body["error"], "unsupported_grant_type");
    let request = axum::http::Request::post("/oauth/token")
        .header("content-type", "application/x-www-form-urlencoded")
        .header("authorization", "Basic YTpi")
        .body(axum::body::Body::from(format!(
            "grant_type=refresh_token&client_id={client_id}"
        )))?;
    let basic =
        crate::support::read(tower::ServiceExt::oneshot(app.router.clone(), request).await?)
            .await?;
    assert_eq!(basic.status, StatusCode::UNAUTHORIZED);
    Ok(())
}

#[sqlx::test]
async fn the_device_flow_issues_tokens_after_approval(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    let client_id = register(&app, REDIRECT).await?;
    let start = app
        .form(
            "/oauth/device",
            &[("client_id", &client_id), ("resource", RESOURCE)],
        )
        .await?;
    assert_eq!(start.status, StatusCode::OK, "{:?}", start.body);
    let device_code = start.body["device_code"].as_str().ok_or("no device code")?;
    let user_code = start.body["user_code"].as_str().ok_or("no user code")?;
    assert_eq!(
        start.body["verification_uri"],
        "http://localhost:8080/device"
    );
    assert_eq!(start.body["interval"], 5);

    let poll = |code: &str| {
        let pairs = [
            (
                "grant_type".to_owned(),
                "urn:ietf:params:oauth:grant-type:device_code".to_owned(),
            ),
            ("client_id".to_owned(), client_id.clone()),
            ("device_code".to_owned(), code.to_owned()),
        ];
        let app = &app;
        async move {
            let borrowed: Vec<(&str, &str)> = pairs
                .iter()
                .map(|(k, v)| (k.as_str(), v.as_str()))
                .collect();
            app.form("/oauth/token", &borrowed).await
        }
    };
    let pending = poll(device_code).await?;
    assert_eq!(pending.body["error"], "authorization_pending");
    let fast = poll(device_code).await?;
    assert_eq!(fast.body["error"], "slow_down");

    let view = admin
        .get(&format!("/api/oauth/consents/{user_code}"))
        .await?;
    assert_eq!(view.status, StatusCode::OK, "{:?}", view.body);
    assert_eq!(view.body["kind"], "device");
    let typed = user_code.replace('-', "").to_lowercase();
    let approved = approve(&mut admin, &typed, json!({"name": "cli"})).await?;
    assert_eq!(approved.status, StatusCode::OK, "{:?}", approved.body);

    sqlx::query("UPDATE device_codes SET last_polled_at = now() - interval '1 minute'")
        .execute(&app.pool)
        .await?;
    let reply = poll(device_code).await?;
    let granted = grant(&reply)?;
    assert_eq!(admin.get("/api/connections").await?.body[0]["name"], "cli");
    let again = poll(device_code).await?;
    assert_eq!(
        again.body["error"], "invalid_grant",
        "device codes are single use"
    );
    assert_ne!(granted.access, "");
    Ok(())
}

#[sqlx::test]
async fn device_codes_can_be_denied_expire_and_belong_to_one_client(pool: PgPool) -> TestResult {
    let app = App::new(pool.clone());
    let (admin, _) = app.admin().await?;
    let client_id = register(&app, REDIRECT).await?;
    let other = register(&app, REDIRECT).await?;
    let grant_type = "urn:ietf:params:oauth:grant-type:device_code";

    let denied = app
        .form(
            "/oauth/device",
            &[("client_id", &client_id), ("resource", RESOURCE)],
        )
        .await?;
    let user_code = denied.body["user_code"].as_str().ok_or("no code")?;
    let device_code = denied.body["device_code"].as_str().ok_or("no code")?;
    let reply = admin
        .post(&format!("/api/oauth/consents/{user_code}/deny"), json!({}))
        .await?;
    assert_eq!(reply.status, StatusCode::OK);
    let poll = app
        .form(
            "/oauth/token",
            &[
                ("grant_type", grant_type),
                ("client_id", &client_id),
                ("device_code", device_code),
            ],
        )
        .await?;
    assert_eq!(poll.body["error"], "access_denied");

    let expiring = app
        .form(
            "/oauth/device",
            &[("client_id", &client_id), ("resource", RESOURCE)],
        )
        .await?;
    let device_code = expiring.body["device_code"].as_str().ok_or("no code")?;
    sqlx::query("UPDATE device_codes SET created_at = now() - interval '2 hours', expires_at = now() - interval '1 hour'")
        .execute(&pool)
        .await?;
    let poll = app
        .form(
            "/oauth/token",
            &[
                ("grant_type", grant_type),
                ("client_id", &client_id),
                ("device_code", device_code),
            ],
        )
        .await?;
    assert_eq!(poll.body["error"], "expired_token");

    let mine = app
        .form(
            "/oauth/device",
            &[("client_id", &client_id), ("resource", RESOURCE)],
        )
        .await?;
    let device_code = mine.body["device_code"].as_str().ok_or("no code")?;
    let stolen = app
        .form(
            "/oauth/token",
            &[
                ("grant_type", grant_type),
                ("client_id", &other),
                ("device_code", device_code),
            ],
        )
        .await?;
    assert_eq!(stolen.body["error"], "invalid_grant");
    Ok(())
}

#[sqlx::test]
async fn the_consent_view_flags_a_name_that_does_not_match_the_redirect_host(
    pool: PgPool,
) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let mut flags = Vec::new();
    for (name, redirect) in [
        ("Claude", "https://claude.ai/callback"),
        ("Claude", "https://phish.example/callback"),
    ] {
        let registered = app
            .router_post_json(
                "/oauth/register",
                json!({"client_name": name, "redirect_uris": [redirect]}),
            )
            .await?;
        let client_id = registered.body["client_id"].as_str().ok_or("no id")?;
        let reply = app
            .plain_get(&authorize_path(client_id, redirect, &[]))
            .await?;
        let reference = param(&location(&reply)?, "consent").ok_or("no consent")?;
        let view = admin
            .get(&format!("/api/oauth/consents/{reference}"))
            .await?;
        assert_eq!(view.body["client"]["unverified"], true);
        flags.push((
            view.body["redirect_host"].clone(),
            view.body["name_matches_redirect"].clone(),
        ));
    }
    assert_eq!(
        flags,
        [
            (json!("claude.ai"), json!(true)),
            (json!("phish.example"), json!(false)),
        ]
    );
    Ok(())
}
