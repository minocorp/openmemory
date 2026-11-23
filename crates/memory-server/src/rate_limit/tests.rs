use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use axum::middleware::from_fn_with_state;
use axum::routing::{get, post};
use tower::ServiceExt;

use super::*;

const PROXY: &str = "10.0.0.1:4000";

fn app() -> Router {
    app_with(RateLimits::behind(vec!["10.0.0.0/8".parse().unwrap()]))
}

fn app_with(limits: RateLimits) -> Router {
    Router::new()
        .route("/api/auth/login/start", post(|| async { "ok" }))
        .route("/oauth/token", post(|| async { "ok" }))
        .route("/oauth/register", post(|| async { "ok" }))
        .route("/oauth/device", post(|| async { "ok" }))
        .route("/oauth/authorize", get(|| async { "ok" }))
        .route("/oauth/revoke", post(|| async { "ok" }))
        .route("/api/upload-links/{token}", post(|| async { "ok" }))
        .route("/mcp", get(|| async { "ok" }))
        .layer(from_fn_with_state(limits, enforce))
}

async fn hit_from(app: &Router, peer: &str, method: &str, path: &str, forwarded: &str) -> Response {
    let mut request = Request::builder()
        .method(method)
        .uri(path)
        .header(FORWARDED_FOR, forwarded)
        .body(Body::empty())
        .unwrap();
    request
        .extensions_mut()
        .insert(ConnectInfo(peer.parse::<SocketAddr>().unwrap()));
    app.clone().oneshot(request).await.unwrap()
}

async fn hit(app: &Router, method: &str, path: &str, forwarded: &str) -> Response {
    hit_from(app, PROXY, method, path, forwarded).await
}

async fn exhaust(app: &Router, method: &str, path: &str, forwarded: &str) -> Response {
    let mut last = hit(app, method, path, forwarded).await;
    for _ in 0..70 {
        if last.status() == StatusCode::TOO_MANY_REQUESTS {
            break;
        }
        last = hit(app, method, path, forwarded).await;
    }
    last
}

#[tokio::test]
async fn each_limited_endpoint_answers_429_with_retry_after_once_the_burst_is_spent() {
    let app = app();
    for (method, path) in [
        ("POST", "/api/auth/login/start"),
        ("POST", "/oauth/token"),
        ("POST", "/oauth/register"),
        ("POST", "/oauth/device"),
        ("GET", "/oauth/authorize"),
        ("POST", "/oauth/revoke"),
        ("POST", "/api/upload-links/abc"),
    ] {
        let response = exhaust(&app, method, path, "203.0.113.7").await;
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS, "{path}");
        assert!(response.headers().contains_key(RETRY_AFTER), "{path}");
    }
}

#[tokio::test]
async fn other_clients_and_other_paths_are_not_affected() {
    let app = app();
    exhaust(&app, "POST", "/oauth/register", "203.0.113.7").await;
    let other_client = hit(&app, "POST", "/oauth/register", "203.0.113.8").await;
    let other_path = hit(&app, "POST", "/oauth/token", "203.0.113.7").await;
    let unlimited = exhaust(&app, "GET", "/mcp", "203.0.113.7").await;
    assert_eq!(other_client.status(), StatusCode::OK);
    assert_eq!(other_path.status(), StatusCode::OK);
    assert_eq!(unlimited.status(), StatusCode::OK);
}

#[tokio::test]
async fn the_last_forwarded_entry_is_the_client() {
    let app = app();
    exhaust(&app, "POST", "/oauth/register", "198.51.100.1, 203.0.113.7").await;
    let spoofed_first_entry = hit(&app, "POST", "/oauth/register", "192.0.2.1, 203.0.113.7").await;
    assert_eq!(spoofed_first_entry.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn ipv6_clients_share_a_bucket_per_64_bit_prefix() {
    let app = app();
    exhaust(&app, "POST", "/oauth/register", "2001:db8:1:2::1").await;
    let same_prefix = hit(&app, "POST", "/oauth/register", "2001:db8:1:2:ffff::9").await;
    let other_prefix = hit(&app, "POST", "/oauth/register", "2001:db8:1:3::1").await;
    assert_eq!(same_prefix.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(other_prefix.status(), StatusCode::OK);
}

#[tokio::test]
async fn forwarded_entries_are_ignored_when_the_peer_is_not_a_trusted_proxy() {
    let app = app();
    let peer = "198.51.100.9:5000";
    for _ in 0..70 {
        hit_from(&app, peer, "POST", "/oauth/register", "192.0.2.1").await;
    }
    let rotated = hit_from(&app, peer, "POST", "/oauth/register", "192.0.2.2").await;
    assert_eq!(rotated.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn without_trusted_proxies_the_socket_peer_is_the_client() {
    let app = app_with(RateLimits::new());
    for _ in 0..70 {
        hit(&app, "POST", "/oauth/register", "203.0.113.7").await;
    }
    let spoofed = hit(&app, "POST", "/oauth/register", "203.0.113.8").await;
    assert_eq!(spoofed.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn chained_trusted_proxies_are_skipped_to_find_the_client() {
    let app = app();
    exhaust(&app, "POST", "/oauth/register", "203.0.113.7, 10.0.0.2").await;
    let same_client = hit(
        &app,
        "POST",
        "/oauth/register",
        "192.0.2.1, 203.0.113.7, 10.0.0.2",
    )
    .await;
    assert_eq!(same_client.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn an_unparsable_forwarded_entry_falls_back_to_the_peer() {
    let app = app();
    for _ in 0..70 {
        hit(&app, "POST", "/oauth/register", "garbage").await;
    }
    let other = hit(&app, "POST", "/oauth/register", "also garbage").await;
    assert_eq!(other.status(), StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn forwarded_headers_from_an_internal_peer_without_trusted_proxies_warn_once() {
    let limits = RateLimits::new();
    let app = app_with(limits.clone());
    assert!(!limits.warned_unconfigured_proxy.load(Ordering::Relaxed));
    hit_from(
        &app,
        "203.0.113.5:4000",
        "POST",
        "/oauth/token",
        "198.51.100.1",
    )
    .await;
    assert!(!limits.warned_unconfigured_proxy.load(Ordering::Relaxed));
    hit_from(
        &app,
        "127.0.0.1:4000",
        "POST",
        "/oauth/token",
        "198.51.100.1",
    )
    .await;
    assert!(limits.warned_unconfigured_proxy.load(Ordering::Relaxed));
}

#[tokio::test]
async fn trusted_proxies_never_trigger_the_warning() {
    let limits = RateLimits::behind(vec!["10.0.0.0/8".parse().unwrap()]);
    let app = app_with(limits.clone());
    hit(&app, "POST", "/oauth/token", "198.51.100.1").await;
    assert!(!limits.warned_unconfigured_proxy.load(Ordering::Relaxed));
}

const CLIENT_HEADER: &str = "cf-connecting-ip";

fn header_app() -> Router {
    app_with(
        RateLimits::behind(vec!["10.0.0.0/8".parse().unwrap()])
            .reading_client_ip_from(Some(HeaderName::from_static(CLIENT_HEADER))),
    )
}

async fn register(app: &Router, peer: &str, headers: &[(&str, &str)]) -> StatusCode {
    let mut builder = Request::builder().method("POST").uri("/oauth/register");
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let mut request = builder.body(Body::empty()).unwrap();
    request
        .extensions_mut()
        .insert(ConnectInfo(peer.parse::<SocketAddr>().unwrap()));
    app.clone().oneshot(request).await.unwrap().status()
}

async fn spend(app: &Router, peer: &str, headers: &[(&str, &str)]) {
    for _ in 0..70 {
        register(app, peer, headers).await;
    }
}

#[tokio::test]
async fn a_trusted_peer_gets_its_client_from_the_configured_header() {
    let app = header_app();
    let forwarded = ("x-forwarded-for", "192.0.2.1");
    spend(&app, PROXY, &[(CLIENT_HEADER, "203.0.113.7"), forwarded]).await;
    let same_client = register(&app, PROXY, &[(CLIENT_HEADER, "203.0.113.7")]).await;
    let other_client = register(&app, PROXY, &[(CLIENT_HEADER, "203.0.113.8"), forwarded]).await;
    assert_eq!(same_client, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(other_client, StatusCode::OK);
}

#[tokio::test]
async fn an_untrusted_peer_cannot_choose_its_client_with_the_configured_header() {
    let app = header_app();
    let peer = "198.51.100.9:5000";
    spend(&app, peer, &[(CLIENT_HEADER, "203.0.113.7")]).await;
    let rotated = register(&app, peer, &[(CLIENT_HEADER, "203.0.113.8")]).await;
    assert_eq!(rotated, StatusCode::TOO_MANY_REQUESTS);
}

#[tokio::test]
async fn a_malformed_or_missing_client_header_falls_back_to_forwarded_for_then_the_peer() {
    let app = header_app();
    spend(
        &app,
        PROXY,
        &[
            (CLIENT_HEADER, "203.0.113.7, 192.0.2.1"),
            ("x-forwarded-for", "203.0.113.9"),
        ],
    )
    .await;
    let via_forwarded = register(&app, PROXY, &[("x-forwarded-for", "203.0.113.9")]).await;
    assert_eq!(via_forwarded, StatusCode::TOO_MANY_REQUESTS);
    spend(&app, PROXY, &[(CLIENT_HEADER, "garbage")]).await;
    let via_peer = register(&app, PROXY, &[(CLIENT_HEADER, "also garbage")]).await;
    assert_eq!(via_peer, StatusCode::TOO_MANY_REQUESTS);
}
