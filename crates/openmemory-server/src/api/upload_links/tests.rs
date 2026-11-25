use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request, StatusCode};
use object_store::memory::InMemory;
use serde_json::Value;
use sqlx::PgPool;
use tower::ServiceExt;

use crate::access::Actor;
use crate::state::AppState;
use crate::test_support::agent;

struct Fixture {
    router: Router,
    state: AppState,
    actor: Actor,
}

async fn fixture(pool: PgPool) -> Fixture {
    let actor = agent(&pool, "mino", "claude-code").await;
    sqlx::query!("UPDATE connections SET scopes = ARRAY['*']")
        .execute(&pool)
        .await
        .unwrap();
    let state = AppState::new(pool, Arc::new(InMemory::new()), 64);
    Fixture {
        router: crate::router(state.clone()),
        state,
        actor,
    }
}

async fn send(
    fixture: &Fixture,
    method: Method,
    token: &str,
    content_type: Option<&str>,
    body: &'static [u8],
) -> (StatusCode, Value) {
    let mut request = Request::builder()
        .method(method)
        .uri(format!("/api/upload-links/{token}"));
    if let Some(content_type) = content_type {
        request = request.header("content-type", content_type);
    }
    let response = fixture
        .router
        .clone()
        .oneshot(request.body(Body::from(body)).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (
        status,
        serde_json::from_slice(&bytes).unwrap_or(Value::Null),
    )
}

#[sqlx::test]
async fn a_link_is_visible_and_redeemable_once_without_a_session(pool: PgPool) {
    let fx = fixture(pool).await;
    let link = fx
        .state
        .files()
        .create_upload_link(&fx.actor, "inbox/receipt.txt")
        .await
        .unwrap();

    let (status, view) = send(&fx, Method::GET, &link.token, None, b"").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(view["path"], "inbox/receipt.txt");
    assert!(view["expires_at"].is_string());

    let (status, file) = send(
        &fx,
        Method::POST,
        &link.token,
        Some("text/plain"),
        b"lunch 12,000",
    )
    .await;
    assert_eq!(status, StatusCode::CREATED, "{file}");
    assert_eq!(file["path"], "inbox/receipt.txt");
    assert_eq!(file["size"], 12);

    let (status, spent) = send(&fx, Method::GET, &link.token, None, b"").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(spent["type"], "urn:openmemory:problem:link_invalid");
    let (status, _) = send(&fx, Method::POST, &link.token, Some("text/plain"), b"again").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn unknown_tokens_are_not_found(pool: PgPool) {
    let fx = fixture(pool).await;
    let (status, body) = send(&fx, Method::GET, "nope", None, b"").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["type"], "urn:openmemory:problem:link_invalid");
    let (status, _) = send(&fx, Method::POST, "nope", Some("text/plain"), b"x").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn a_rejected_upload_leaves_the_link_usable(pool: PgPool) {
    let fx = fixture(pool).await;
    let link = fx
        .state
        .files()
        .create_upload_link(&fx.actor, "a.txt")
        .await
        .unwrap();

    let (status, _) = send(
        &fx,
        Method::POST,
        &link.token,
        Some("application/zip"),
        b"PK",
    )
    .await;
    assert_eq!(status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    let (status, _) = send(&fx, Method::POST, &link.token, None, b"x").await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let too_big: &'static [u8] = &[b'x'; 100];
    let (status, body) = send(&fx, Method::POST, &link.token, Some("text/plain"), too_big).await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(body["limit"], 64);

    let (status, _) = send(&fx, Method::POST, &link.token, Some("text/plain"), b"fine").await;
    assert_eq!(status, StatusCode::CREATED);
}

#[sqlx::test]
async fn the_token_is_checked_before_the_body_is_considered(pool: PgPool) {
    let fx = fixture(pool).await;
    let (status, body) = send(&fx, Method::POST, "nope", None, b"x").await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    assert_eq!(body["type"], "urn:openmemory:problem:link_invalid");
    let too_big: &'static [u8] = &[b'x'; 100];
    let (status, _) = send(&fx, Method::POST, "nope", Some("text/plain"), too_big).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

#[sqlx::test]
async fn a_declared_length_over_the_cap_is_refused_without_reading(pool: PgPool) {
    let fx = fixture(pool).await;
    let link = fx
        .state
        .files()
        .create_upload_link(&fx.actor, "a.txt")
        .await
        .unwrap();
    let request = Request::post(format!("/api/upload-links/{}", link.token))
        .header("content-type", "text/plain")
        .header("content-length", "100")
        .body(Body::from("x"))
        .unwrap();
    let response = fx.router.clone().oneshot(request).await.unwrap();
    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[sqlx::test]
async fn upload_bodies_beyond_the_concurrency_cap_are_turned_away(pool: PgPool) {
    let fx = fixture(pool).await;
    let link = fx
        .state
        .files()
        .create_upload_link(&fx.actor, "a.txt")
        .await
        .unwrap();
    let mut held = Vec::new();
    while let Some(permit) = fx.state.files().reserve_upload_body() {
        held.push(permit);
    }
    let (status, body) = send(&fx, Method::POST, &link.token, Some("text/plain"), b"x").await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["type"], "urn:openmemory:problem:busy");
    held.pop();
    let (status, _) = send(&fx, Method::POST, &link.token, Some("text/plain"), b"x").await;
    assert_eq!(status, StatusCode::CREATED);
}
