use axum::http::StatusCode;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::json;
use sqlx::PgPool;

use crate::support::{App, TestResult};

async fn handle_of(pool: &PgPool, username: &str) -> TestResult<Vec<u8>> {
    Ok(
        sqlx::query_scalar("SELECT user_handle FROM users WHERE username = $1")
            .bind(username)
            .fetch_one(pool)
            .await?,
    )
}

#[sqlx::test]
async fn registration_sends_the_webauthn_user_entity_and_asks_for_a_discoverable_credential(
    pool: PgPool,
) -> TestResult {
    let app = App::new(pool.clone());
    let link = app.state.bootstrap_link().await?.ok_or("no link")?;
    let token = link.split_once("token=").ok_or("no token")?.1.to_owned();
    let mut browser = app.browser();
    let start = browser
        .post(
            "/api/auth/bootstrap/start",
            json!({"token": token, "username": "Mino.Kim", "display_name": "민호 Kim"}),
        )
        .await?;
    assert_eq!(start.status, StatusCode::OK, "{:?}", start.body);
    let public_key = &start.body["options"]["publicKey"];
    assert_eq!(public_key["user"]["name"], "mino.kim");
    assert_eq!(public_key["user"]["displayName"], "민호 Kim");
    let handle = URL_SAFE_NO_PAD.decode(public_key["user"]["id"].as_str().ok_or("no id")?)?;
    assert_eq!(handle.len(), 16);
    assert!(!String::from_utf8_lossy(&handle).contains("mino"));
    let selection = &public_key["authenticatorSelection"];
    assert_eq!(selection["residentKey"], "required");
    assert_eq!(selection["requireResidentKey"], true);
    assert_eq!(selection["userVerification"], "required");

    let credential = browser.register_credential(&start.body["options"])?;
    let finish = browser
        .post(
            "/api/auth/bootstrap/finish",
            json!({"challenge": start.body["challenge"], "credential": credential}),
        )
        .await?;
    assert_eq!(finish.status, StatusCode::OK, "{:?}", finish.body);
    assert_eq!(handle_of(&pool, "mino.kim").await?, handle);
    Ok(())
}

#[sqlx::test]
async fn every_user_gets_a_different_handle_and_adding_a_passkey_reuses_it(
    pool: PgPool,
) -> TestResult {
    let app = App::new(pool.clone());
    let (admin, _) = app.admin().await?;
    app.member(&admin, "member").await?;
    assert_ne!(
        handle_of(&pool, "admin").await?,
        handle_of(&pool, "member").await?
    );

    let start = admin.start_add_passkey("laptop").await?;
    assert_eq!(start.status, StatusCode::OK, "{:?}", start.body);
    let user = &start.body["options"]["publicKey"]["user"];
    assert_eq!(user["name"], "admin");
    assert_eq!(user["displayName"], "Display admin");
    let handle = URL_SAFE_NO_PAD.decode(user["id"].as_str().ok_or("no id")?)?;
    assert_eq!(handle, handle_of(&pool, "admin").await?);
    Ok(())
}

#[sqlx::test]
async fn usernames_are_lowercase_and_unique_while_display_names_may_repeat(
    pool: PgPool,
) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let invite = admin.create_invite().await?;
    let token = invite.body["token"].clone();

    for bad in ["has space", "slash/name", "민호", "", &"a".repeat(33)] {
        let reply = app
            .browser()
            .post(
                "/api/auth/register/start",
                json!({"token": token, "username": bad, "display_name": "Someone"}),
            )
            .await?;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{bad}");
    }
    let taken = app
        .browser()
        .post(
            "/api/auth/register/start",
            json!({"token": token, "username": "ADMIN", "display_name": "Someone"}),
        )
        .await?;
    assert_eq!(taken.status, StatusCode::CONFLICT);

    let mut member = app.browser();
    let start = member
        .post(
            "/api/auth/register/start",
            json!({"token": token, "username": "member", "display_name": "Display admin"}),
        )
        .await?;
    assert_eq!(start.status, StatusCode::OK, "{:?}", start.body);
    let credential = member.register_credential(&start.body["options"])?;
    let finish = member
        .post(
            "/api/auth/register/finish",
            json!({"challenge": start.body["challenge"], "credential": credential}),
        )
        .await?;
    assert_eq!(finish.status, StatusCode::OK, "{:?}", finish.body);
    member.adopt_cookie(&finish);
    let me = member.get("/api/auth/me").await?;
    assert_eq!(me.body["username"], "member");
    assert_eq!(me.body["display_name"], "Display admin");
    Ok(())
}

#[sqlx::test]
async fn login_by_typing_a_username_ignores_case(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    assert_eq!(admin.login("Admin").await?.status, StatusCode::OK);
    Ok(())
}

#[sqlx::test]
async fn a_discoverable_login_signs_in_by_user_handle_without_a_username(
    pool: PgPool,
) -> TestResult {
    let app = App::new(pool.clone());
    let (mut admin, _) = app.admin().await?;
    let handle = handle_of(&pool, "admin").await?;
    let login = admin.login_discoverable(&handle).await?;
    assert_eq!(login.status, StatusCode::OK, "{:?}", login.body);
    let me = admin.get("/api/auth/me").await?;
    assert_eq!(me.body["username"], "admin");
    let used: Option<time::OffsetDateTime> =
        sqlx::query_scalar("SELECT last_used_at FROM passkeys")
            .fetch_one(&pool)
            .await?;
    assert!(used.is_some());
    Ok(())
}

#[sqlx::test]
async fn a_discoverable_login_fails_for_an_unknown_handle_and_starts_a_conditional_request(
    pool: PgPool,
) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    let unknown = admin.login_discoverable(&[7_u8; 16]).await?;
    assert_eq!(unknown.status, StatusCode::UNAUTHORIZED);

    let start = admin
        .post("/api/auth/login/discoverable/start", json!({}))
        .await?;
    assert_eq!(
        start.body["options"]["mediation"], "conditional",
        "{:?}",
        start.body
    );
    assert_eq!(
        start.body["options"]["publicKey"]["allowCredentials"],
        json!([])
    );
    Ok(())
}

#[sqlx::test]
async fn challenges_do_not_cross_between_the_two_login_flows(pool: PgPool) -> TestResult {
    let app = App::new(pool.clone());
    let (admin, _) = app.admin().await?;
    let handle = handle_of(&pool, "admin").await?;

    let typed = admin
        .post("/api/auth/login/start", json!({"username": "admin"}))
        .await?;
    let typed_credential = admin.assert_credential(&typed.body["options"])?;
    let crossed = admin
        .post(
            "/api/auth/login/discoverable/finish",
            json!({"challenge": typed.body["challenge"], "credential": typed_credential}),
        )
        .await?;
    assert_eq!(crossed.status, StatusCode::UNAUTHORIZED);

    let found = admin
        .post("/api/auth/login/discoverable/start", json!({}))
        .await?;
    let found_credential = admin.assert_discoverable(&found.body["options"], &handle)?;
    let crossed = admin
        .post(
            "/api/auth/login/finish",
            json!({"challenge": found.body["challenge"], "credential": found_credential}),
        )
        .await?;
    assert_eq!(crossed.status, StatusCode::UNAUTHORIZED);

    let approval = admin
        .post("/api/auth/confirm", json!({"action": "export"}))
        .await?;
    let approval_credential = admin.assert_credential(&approval.body["options"])?;
    let crossed = admin
        .post(
            "/api/auth/login/discoverable/finish",
            json!({"challenge": approval.body["challenge"], "credential": approval_credential}),
        )
        .await?;
    assert_eq!(crossed.status, StatusCode::UNAUTHORIZED);
    Ok(())
}

#[sqlx::test]
async fn users_change_their_own_username_and_display_name(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let member = app.member(&admin, "member").await?;

    let renamed = member
        .patch(
            "/api/auth/me",
            json!({"username": "Member.Two", "display_name": "Member Two"}),
        )
        .await?;
    assert_eq!(renamed.status, StatusCode::OK, "{:?}", renamed.body);
    assert_eq!(renamed.body["username"], "member.two");
    assert_eq!(renamed.body["display_name"], "Member Two");

    let taken = member
        .patch("/api/auth/me", json!({"username": "admin"}))
        .await?;
    assert_eq!(taken.status, StatusCode::CONFLICT);
    let empty = member.patch("/api/auth/me", json!({})).await?;
    assert_eq!(empty.status, StatusCode::BAD_REQUEST);

    let typed = app
        .browser()
        .post("/api/auth/login/start", json!({"username": "member.two"}))
        .await?;
    assert_eq!(typed.status, StatusCode::OK);
    Ok(())
}

#[sqlx::test]
async fn only_admins_change_other_users_profiles(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let member = app.member(&admin, "member").await?;
    let users = admin.get("/api/users").await?;
    let member_id = users.body[1]["id"].as_i64().ok_or("no id")?;
    let admin_id = users.body[0]["id"].as_i64().ok_or("no id")?;

    let forbidden = member
        .patch(
            &format!("/api/users/{admin_id}/profile"),
            json!({"display_name": "Boss"}),
        )
        .await?;
    assert_eq!(forbidden.status, StatusCode::FORBIDDEN);

    let changed = admin
        .patch(
            &format!("/api/users/{member_id}/profile"),
            json!({"username": "kim", "display_name": "Kim"}),
        )
        .await?;
    assert_eq!(changed.status, StatusCode::OK, "{:?}", changed.body);
    assert_eq!(changed.body["username"], "kim");
    assert_eq!(
        member.get("/api/auth/me").await?.body["display_name"],
        "Kim"
    );

    let taken = admin
        .patch(
            &format!("/api/users/{member_id}/profile"),
            json!({"username": "admin"}),
        )
        .await?;
    assert_eq!(taken.status, StatusCode::CONFLICT);
    let missing = admin
        .patch("/api/users/9999/profile", json!({"display_name": "x"}))
        .await?;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
    Ok(())
}
