use std::sync::Arc;

use axum::body::Body;
use axum::http::header::{
    CACHE_CONTROL, CONTENT_SECURITY_POLICY, REFERRER_POLICY, STRICT_TRANSPORT_SECURITY,
    X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS,
};
use axum::http::{HeaderMap, Request};
use object_store::memory::InMemory;
use sqlx::PgPool;
use tower::ServiceExt;

use crate::state::AppState;

async fn headers_of(pool: PgPool, public_url: &str, path: &str) -> HeaderMap {
    let state = AppState::new(pool, Arc::new(InMemory::new()), 64)
        .with_public_url(public_url.parse().unwrap())
        .unwrap();
    crate::router(state)
        .oneshot(Request::get(path).body(Body::empty()).unwrap())
        .await
        .unwrap()
        .headers()
        .clone()
}

fn value<'a>(headers: &'a HeaderMap, name: &axum::http::HeaderName) -> &'a str {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
}

#[sqlx::test]
async fn every_response_carries_the_hardening_headers(pool: PgPool) {
    let headers = headers_of(pool, "https://memory.example", "/healthz").await;
    assert_eq!(value(&headers, &X_FRAME_OPTIONS), "DENY");
    assert_eq!(value(&headers, &REFERRER_POLICY), "no-referrer");
    assert_eq!(value(&headers, &X_CONTENT_TYPE_OPTIONS), "nosniff");
    let policy = value(&headers, &CONTENT_SECURITY_POLICY);
    assert!(policy.contains("frame-ancestors 'none'"));
    assert!(policy.contains("script-src 'self'"));
    assert!(policy.contains("connect-src 'self'"));
    assert!(policy.contains("default-src 'none'"));
}

#[sqlx::test]
async fn hsts_is_sent_only_for_an_https_public_url(pool: PgPool) {
    let secure = headers_of(pool.clone(), "https://memory.example", "/healthz").await;
    assert!(value(&secure, &STRICT_TRANSPORT_SECURITY).starts_with("max-age="));
    let local = headers_of(pool, "http://localhost:8080", "/healthz").await;
    assert!(!local.contains_key(STRICT_TRANSPORT_SECURITY));
}

#[sqlx::test]
async fn auth_oauth_consent_and_upload_link_responses_are_not_cacheable(pool: PgPool) {
    for path in [
        "/api/auth/me",
        "/oauth/authorize",
        "/api/oauth/consents/x",
        "/api/upload-links/x",
    ] {
        let headers = headers_of(pool.clone(), "https://memory.example", path).await;
        assert_eq!(value(&headers, &CACHE_CONTROL), "no-store", "{path}");
    }
    let page = headers_of(pool, "https://memory.example", "/healthz").await;
    assert!(!page.contains_key(CACHE_CONTROL));
}
