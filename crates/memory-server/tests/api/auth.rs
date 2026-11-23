use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use std::time::Duration;

use serde_json::{Value, json};
use sqlx::PgPool;

use crate::support::{App, TestResult};

#[sqlx::test]
async fn bootstrap_creates_the_first_admin_with_a_session_and_recovery_code(
    pool: PgPool,
) -> TestResult {
    let app = App::new(pool);
    let (admin, recovery) = app.admin().await?;
    assert_eq!(recovery.len(), 39);
    let me = admin.get("/api/auth/me").await?;
    assert_eq!(me.status, StatusCode::OK);
    assert_eq!(me.body["username"], "admin");
    assert_eq!(me.body["display_name"], "Display admin");
    assert_eq!(me.body["is_admin"], true);
    assert!(app.state.bootstrap_link().await?.is_none());
    Ok(())
}

#[sqlx::test]
async fn bootstrap_token_is_checked_and_works_once(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let link = app.state.bootstrap_link().await?.ok_or("no link")?;
    let token = link.split_once("token=").ok_or("no token")?.1.to_owned();
    let browser = app.browser();
    let wrong = browser
        .post(
            "/api/auth/bootstrap/start",
            json!({"token": "wrong", "username": "x", "display_name": "Someone"}),
        )
        .await?;
    assert_eq!(wrong.status, StatusCode::BAD_REQUEST);

    let mut first = app.browser();
    first.sign_up("bootstrap", &token, "admin").await?;
    let again = browser
        .post(
            "/api/auth/bootstrap/start",
            json!({"token": token, "username": "other", "display_name": "Someone"}),
        )
        .await?;
    assert_eq!(again.status, StatusCode::BAD_REQUEST);
    Ok(())
}

#[sqlx::test]
async fn session_cookie_is_host_prefixed_secure_and_http_only(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let link = app.state.bootstrap_link().await?.ok_or("no link")?;
    let token = link.split_once("token=").ok_or("no token")?.1.to_owned();
    let mut browser = app.browser();
    let start = browser
        .post(
            "/api/auth/bootstrap/start",
            json!({"token": token, "username": "admin", "display_name": "Someone"}),
        )
        .await?;
    let credential = browser.register_credential(&start.body["options"])?;
    let finish = browser
        .post(
            "/api/auth/bootstrap/finish",
            json!({"challenge": start.body["challenge"], "credential": credential}),
        )
        .await?;
    let cookie = finish.headers["set-cookie"].to_str()?;
    assert!(cookie.starts_with("__Host-session="));
    for attribute in ["Secure", "HttpOnly", "SameSite=Lax", "Path=/"] {
        assert!(cookie.contains(attribute), "{cookie}");
    }
    assert!(!cookie.contains("Domain"));
    Ok(())
}

#[sqlx::test]
async fn logging_in_and_out_manages_the_session(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    let stale = admin.clone_cookie();
    let logout = admin.post("/api/auth/logout", json!({})).await?;
    assert_eq!(logout.status, StatusCode::NO_CONTENT);
    assert_eq!(
        admin.get_with_cookie("/api/auth/me", &stale).await?.status,
        StatusCode::UNAUTHORIZED
    );

    let login = admin.login("admin").await?;
    assert_eq!(login.status, StatusCode::OK);
    assert_eq!(admin.get("/api/auth/me").await?.status, StatusCode::OK);
    Ok(())
}

#[sqlx::test]
async fn login_with_an_unknown_authenticator_is_rejected(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    app.admin().await?;
    let mut stranger = app.browser();
    let start = stranger
        .post("/api/auth/login/start", json!({"username": "admin"}))
        .await?;
    assert_eq!(start.status, StatusCode::OK);
    let rejected = stranger.login("admin").await;
    assert!(rejected.is_err() || rejected?.status != StatusCode::OK);
    Ok(())
}

#[sqlx::test]
async fn a_challenge_can_only_be_finished_once(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let start = admin
        .post("/api/auth/login/start", json!({"username": "admin"}))
        .await?;
    let credential = admin.assert_credential(&start.body["options"])?;
    let body = json!({"challenge": start.body["challenge"], "credential": credential});
    let first = admin.post("/api/auth/login/finish", body.clone()).await?;
    assert_eq!(first.status, StatusCode::OK);
    let replay = admin.post("/api/auth/login/finish", body).await?;
    assert_eq!(replay.status, StatusCode::BAD_REQUEST);
    Ok(())
}

#[sqlx::test]
async fn expired_challenges_are_refused(pool: PgPool) -> TestResult {
    let app = App::new(pool.clone());
    let (admin, _) = app.admin().await?;
    let start = admin
        .post("/api/auth/login/start", json!({"username": "admin"}))
        .await?;
    sqlx::query("UPDATE webauthn_challenges SET created_at = now() - interval '2 hours', expires_at = now() - interval '1 hour'")
        .execute(&pool)
        .await?;
    let credential = admin.assert_credential(&start.body["options"])?;
    let finish = admin
        .post(
            "/api/auth/login/finish",
            json!({"challenge": start.body["challenge"], "credential": credential}),
        )
        .await?;
    assert_eq!(finish.status, StatusCode::BAD_REQUEST);
    Ok(())
}

#[sqlx::test]
async fn invites_are_admin_only_and_single_use(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let invite = admin.create_invite().await?;
    assert_eq!(invite.status, StatusCode::CREATED);
    let token = invite.body["token"].as_str().ok_or("no token")?.to_owned();
    assert_eq!(
        admin
            .get("/api/invites")
            .await?
            .body
            .as_array()
            .map(Vec::len),
        Some(1)
    );

    let mut member = app.browser();
    member.sign_up("register", &token, "member").await?;
    let member_me = member.get("/api/auth/me").await?;
    assert_eq!(member_me.body["is_admin"], false);
    assert_eq!(member.create_invite().await?.status, StatusCode::FORBIDDEN);

    let reuse = app
        .browser()
        .post(
            "/api/auth/register/start",
            json!({"token": token, "username": "second", "display_name": "Someone"}),
        )
        .await?;
    assert_eq!(reuse.status, StatusCode::BAD_REQUEST);
    Ok(())
}

#[sqlx::test]
async fn expired_and_deleted_invites_are_refused(pool: PgPool) -> TestResult {
    let app = App::new(pool.clone());
    let (admin, _) = app.admin().await?;
    let expired = admin.create_invite().await?;
    let deleted = admin.create_invite().await?;
    sqlx::query("UPDATE invites SET created_at = now() - interval '9 days', expires_at = now() - interval '2 days' WHERE id = $1")
        .bind(expired.body["id"].as_i64())
        .execute(&pool)
        .await?;
    let id = deleted.body["id"].as_i64().ok_or("no id")?;
    assert_eq!(
        admin.delete(&format!("/api/invites/{id}")).await?.status,
        StatusCode::NO_CONTENT
    );
    for token in [&expired.body["token"], &deleted.body["token"]] {
        let reply = app
            .browser()
            .post(
                "/api/auth/register/start",
                json!({"token": token, "username": "late", "display_name": "Someone"}),
            )
            .await?;
        assert_eq!(reply.status, StatusCode::BAD_REQUEST);
    }
    Ok(())
}

#[sqlx::test]
async fn taken_names_are_rejected_at_signup(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let invite = admin.create_invite().await?;
    let reply = app
        .browser()
        .post(
            "/api/auth/register/start",
            json!({"token": invite.body["token"], "username": "admin", "display_name": "Someone"}),
        )
        .await?;
    assert_eq!(reply.status, StatusCode::CONFLICT);
    Ok(())
}

#[sqlx::test]
async fn recovery_replaces_access_and_revokes_sessions_and_the_code(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (old_device, code) = app.admin().await?;
    let mut new_device = app.browser();

    let start = new_device
        .post(
            "/api/auth/recovery/start",
            json!({"username": "admin", "code": code}),
        )
        .await?;
    assert_eq!(start.status, StatusCode::OK, "{:?}", start.body);
    let credential = new_device.register_credential(&start.body["options"])?;
    let finish = new_device
        .post(
            "/api/auth/recovery/finish",
            json!({"challenge": start.body["challenge"], "credential": credential}),
        )
        .await?;
    assert_eq!(finish.status, StatusCode::OK, "{:?}", finish.body);
    new_device.adopt_cookie(&finish);
    let fresh_code = finish.body["recovery_code"].as_str().ok_or("no code")?;
    assert_ne!(fresh_code, code);
    let remaining: Vec<(String,)> = sqlx::query_as("SELECT name FROM passkeys")
        .fetch_all(&app.pool)
        .await?;
    assert_eq!(remaining.len(), 1);

    assert_eq!(
        old_device.get("/api/auth/me").await?.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(new_device.get("/api/auth/me").await?.status, StatusCode::OK);
    let reuse = app
        .browser()
        .post(
            "/api/auth/recovery/start",
            json!({"username": "admin", "code": code}),
        )
        .await?;
    assert_eq!(reuse.status, StatusCode::UNAUTHORIZED);
    Ok(())
}

#[sqlx::test]
async fn recovery_with_a_wrong_code_is_rejected(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    app.admin().await?;
    let reply = app
        .browser()
        .post(
            "/api/auth/recovery/start",
            json!({"username": "admin", "code": "0000-0000"}),
        )
        .await?;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);
    Ok(())
}

#[sqlx::test]
async fn a_user_can_hold_several_passkeys_but_never_zero(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    let list = admin.get("/api/auth/passkeys").await?;
    let first = list.body[0]["id"].as_i64().ok_or("no id")?;
    assert_eq!(
        admin.delete_passkey(first).await?.status,
        StatusCode::CONFLICT
    );

    let start = admin.start_add_passkey("laptop").await?;
    assert_eq!(start.status, StatusCode::OK, "{:?}", start.body);
    let credential = admin.register_credential(&start.body["options"])?;
    let finish = admin
        .post(
            "/api/auth/passkeys/finish",
            json!({"challenge": start.body["challenge"], "credential": credential}),
        )
        .await?;
    assert_eq!(finish.status, StatusCode::CREATED, "{:?}", finish.body);
    assert_eq!(
        admin
            .get("/api/auth/passkeys")
            .await?
            .body
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    assert_eq!(
        admin.delete_passkey(first).await?.status,
        StatusCode::NO_CONTENT
    );
    Ok(())
}

#[sqlx::test]
async fn sessions_expire_when_idle_and_when_old(pool: PgPool) -> TestResult {
    let app = App::new(pool.clone());
    let (mut admin, _) = app.admin().await?;
    sqlx::query("UPDATE sessions SET last_seen_at = now() - interval '8 days'")
        .execute(&pool)
        .await?;
    assert_eq!(
        admin.get("/api/auth/me").await?.status,
        StatusCode::UNAUTHORIZED
    );

    admin.login("admin").await?;
    assert_eq!(admin.get("/api/auth/me").await?.status, StatusCode::OK);
    sqlx::query("UPDATE sessions SET created_at = now() - interval '31 days', expires_at = now() - interval '1 day'")
        .execute(&pool)
        .await?;
    assert_eq!(
        admin.get("/api/auth/me").await?.status,
        StatusCode::UNAUTHORIZED
    );
    Ok(())
}

#[sqlx::test]
async fn tokens_are_stored_hashed_only(pool: PgPool) -> TestResult {
    let app = App::new(pool.clone());
    let (admin, recovery) = app.admin().await?;
    let cookie = admin.clone_cookie();
    let secret = cookie.split_once('=').ok_or("no cookie")?.1;
    let stored: Vec<u8> = sqlx::query_scalar("SELECT token_hash FROM sessions")
        .fetch_one(&pool)
        .await?;
    assert_eq!(stored.len(), 32);
    assert_ne!(stored, secret.as_bytes());
    let code: Vec<u8> = sqlx::query_scalar("SELECT code_hash FROM recovery_codes")
        .fetch_one(&pool)
        .await?;
    assert_eq!(code.len(), 32);
    assert_ne!(code, recovery.as_bytes());
    Ok(())
}

#[sqlx::test]
async fn state_changing_requests_need_a_matching_origin_and_json(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let cookie = admin.clone_cookie();
    let build = |origin: Option<&str>, content_type: Option<&str>| -> TestResult<Request<Body>> {
        let mut builder = Request::post("/api/auth/confirm").header("cookie", &cookie);
        if let Some(origin) = origin {
            builder = builder.header("origin", origin);
        }
        if let Some(content_type) = content_type {
            builder = builder.header("content-type", content_type);
        }
        Ok(builder.body(Body::from(r#"{"action":"export"}"#))?)
    };
    let cases = [
        (
            Some("https://evil.example"),
            Some("application/json"),
            StatusCode::FORBIDDEN,
        ),
        (
            Some("http://localhost:8081"),
            Some("application/json"),
            StatusCode::FORBIDDEN,
        ),
        (None, Some("application/json"), StatusCode::FORBIDDEN),
        (
            Some("http://localhost:8080"),
            Some("text/plain"),
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
        ),
        (
            Some("http://localhost:8080"),
            Some("application/x-www-form-urlencoded"),
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
        ),
        (
            Some("http://localhost:8080"),
            None,
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
        ),
        (
            Some("http://localhost:8080"),
            Some("application/json; charset=utf-8"),
            StatusCode::OK,
        ),
    ];
    for (origin, content_type, expected) in cases {
        let reply = admin.send(build(origin, content_type)?).await?;
        assert_eq!(reply.status, expected, "{origin:?} {content_type:?}");
    }
    Ok(())
}

#[sqlx::test]
async fn unauthenticated_state_changes_also_need_the_origin(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    app.admin().await?;
    let request = Request::post("/api/auth/login/start")
        .header("origin", "https://evil.example")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"username":"admin"}"#))?;
    let reply = app.browser().send(request).await?;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    Ok(())
}

fn shape(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            Value::Object(map.iter().map(|(k, v)| (k.clone(), shape(v))).collect())
        }
        Value::Array(items) => Value::Array(items.first().map(shape).into_iter().collect()),
        Value::String(_) => json!("string"),
        Value::Number(_) => json!("number"),
        other => other.clone(),
    }
}

fn garbage_assertion() -> Value {
    json!({
        "id": "AAAA",
        "rawId": "AAAA",
        "type": "public-key",
        "response": {
            "authenticatorData": "AAAA",
            "clientDataJSON": "AAAA",
            "signature": "AAAA",
        },
        "extensions": {},
    })
}

#[sqlx::test]
async fn login_start_looks_the_same_for_unknown_names(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    app.admin().await?;
    let browser = app.browser();
    let known = browser
        .post("/api/auth/login/start", json!({"username": "admin"}))
        .await?;
    let unknown = browser
        .post("/api/auth/login/start", json!({"username": "nobody"}))
        .await?;
    let again = browser
        .post("/api/auth/login/start", json!({"username": "nobody"}))
        .await?;
    for reply in [&known, &unknown, &again] {
        assert_eq!(reply.status, StatusCode::OK, "{:?}", reply.body);
    }
    assert_eq!(shape(&known.body), shape(&unknown.body));
    let ids = |reply: &crate::support::Reply| {
        reply.body["options"]["publicKey"]["allowCredentials"].clone()
    };
    assert_eq!(ids(&unknown), ids(&again));
    assert_ne!(unknown.body["challenge"], again.body["challenge"]);
    Ok(())
}

#[sqlx::test]
async fn a_decoy_challenge_has_the_same_structure_and_values_as_a_real_one(
    pool: PgPool,
) -> TestResult {
    let app = App::new(pool);
    app.admin().await?;
    let browser = app.browser();
    let real = browser
        .post("/api/auth/login/start", json!({"username": "admin"}))
        .await?;
    let decoy = browser
        .post("/api/auth/login/start", json!({"username": "nobody"}))
        .await?;
    let real = &real.body["options"];
    let decoy = &decoy.body["options"];
    assert_eq!(keys(real), keys(decoy));
    let (real, decoy) = (&real["publicKey"], &decoy["publicKey"]);
    assert_eq!(keys(real), keys(decoy));
    for field in ["userVerification", "timeout", "rpId"] {
        assert_eq!(real[field], decoy[field], "{field}");
    }
    assert_eq!(real["userVerification"], "required");
    let real_entry = &real["allowCredentials"][0];
    for entry in decoy["allowCredentials"].as_array().ok_or("no list")? {
        assert_eq!(keys(entry), keys(real_entry));
        assert_eq!(entry["type"], real_entry["type"]);
        assert_eq!(
            entry["id"].as_str().map(str::len),
            real_entry["id"].as_str().map(str::len)
        );
        assert!(
            entry["transports"]
                .as_array()
                .is_some_and(|list| !list.is_empty())
        );
    }
    assert_eq!(
        real["challenge"].as_str().map(str::len),
        decoy["challenge"].as_str().map(str::len)
    );
    Ok(())
}

fn keys(value: &Value) -> Vec<String> {
    let mut keys: Vec<String> = value
        .as_object()
        .map_or_else(Vec::new, |map| map.keys().cloned().collect());
    keys.sort();
    keys
}

#[sqlx::test]
async fn finishing_a_decoy_challenge_fails_like_a_wrong_assertion(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    app.admin().await?;
    let browser = app.browser();
    let mut statuses = Vec::new();
    for name in ["admin", "nobody"] {
        let start = browser
            .post("/api/auth/login/start", json!({"username": name}))
            .await?;
        let finish = browser
            .post(
                "/api/auth/login/finish",
                json!({"challenge": start.body["challenge"], "credential": garbage_assertion()}),
            )
            .await?;
        statuses.push((finish.status, finish.body));
    }
    assert_eq!(statuses[0], statuses[1]);
    assert_eq!(statuses[0].0, StatusCode::UNAUTHORIZED);
    Ok(())
}

#[sqlx::test]
async fn deleting_a_passkey_waits_for_the_users_other_passkey_changes(pool: PgPool) -> TestResult {
    let app = App::new(pool.clone());
    let (mut admin, _) = app.admin().await?;
    let start = admin.start_add_passkey("laptop").await?;
    let credential = admin.register_credential(&start.body["options"])?;
    admin
        .post(
            "/api/auth/passkeys/finish",
            json!({"challenge": start.body["challenge"], "credential": credential}),
        )
        .await?;
    let list = admin.get("/api/auth/passkeys").await?;
    let first = list.body[0]["id"].as_i64().ok_or("no id")?;
    let second = list.body[1]["id"].as_i64().ok_or("no id")?;

    let confirmation = admin.confirmed("remove_passkey", json!({})).await?;
    let mut concurrent = pool.begin().await?;
    sqlx::query("SELECT id FROM passkeys FOR UPDATE")
        .execute(&mut *concurrent)
        .await?;
    let target = format!("/api/auth/passkeys/{first}");
    let request = admin.exchange(Method::DELETE, &target, &[], Some(confirmation));
    let other_delete = async {
        tokio::time::sleep(Duration::from_millis(300)).await;
        sqlx::query("DELETE FROM passkeys WHERE id = $1")
            .bind(second)
            .execute(&mut *concurrent)
            .await?;
        concurrent.commit().await?;
        Ok::<(), sqlx::Error>(())
    };
    let (reply, committed) = tokio::join!(request, other_delete);
    committed?;
    assert_eq!(reply?.status, StatusCode::CONFLICT);
    let remaining = admin.get("/api/auth/passkeys").await?;
    assert_eq!(remaining.body.as_array().map(Vec::len), Some(1));
    Ok(())
}

#[sqlx::test]
async fn sensitive_actions_need_a_fresh_assertion_for_that_action(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let me = admin.get("/api/auth/me").await?.body["id"]
        .as_i64()
        .ok_or("no id")?;

    let unconfirmed = [
        admin.post("/api/invites", json!({})).await?,
        admin
            .post("/api/auth/passkeys/start", json!({"name": "laptop"}))
            .await?,
        admin
            .patch(&format!("/api/users/{me}"), json!({"is_admin": true}))
            .await?,
        admin.post("/api/system/export", json!({})).await?,
    ];
    for reply in unconfirmed {
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    }

    let wrong_action = admin
        .post("/api/invites", admin.confirmed("export", json!({})).await?)
        .await?;
    assert_eq!(wrong_action.status, StatusCode::BAD_REQUEST);

    let body = admin.confirmed("create_invite", json!({})).await?;
    let first = admin.post("/api/invites", body.clone()).await?;
    assert_eq!(first.status, StatusCode::CREATED, "{:?}", first.body);
    let replay = admin.post("/api/invites", body).await?;
    assert_eq!(replay.status, StatusCode::BAD_REQUEST);

    let other = app.member(&admin, "member").await?;
    let borrowed = admin.confirm("create_invite").await?;
    let stolen = other
        .post("/api/invites", json!({"confirmation": borrowed}))
        .await?;
    assert_eq!(stolen.status, StatusCode::FORBIDDEN);
    Ok(())
}
