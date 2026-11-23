use axum::body::to_bytes;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode};

use super::*;

fn lookup(name: &str) -> Option<Asset> {
    let (data, mime): (&'static [u8], &str) = match name {
        "index.html" => (b"<html>app</html>", "text/html"),
        "assets/index-abc123.js" => (b"console.log(1)", "application/javascript"),
        "favicon.svg" => (b"<svg/>", "image/svg+xml"),
        _ => return None,
    };
    Some(Asset {
        data: Cow::Borrowed(data),
        mime: mime.to_owned(),
        hash: [7; 32],
    })
}

async fn get(path: &str) -> (StatusCode, HeaderMap, Vec<u8>) {
    request(&Method::GET, path, &HeaderMap::new()).await
}

async fn request(
    method: &Method,
    path: &str,
    headers: &HeaderMap,
) -> (StatusCode, HeaderMap, Vec<u8>) {
    let response = respond(method, path, headers, lookup);
    let status = response.status();
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, headers, body.to_vec())
}

fn header<'a>(headers: &'a HeaderMap, name: &axum::http::HeaderName) -> &'a str {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
}

#[tokio::test]
async fn the_root_serves_the_index_without_caching() {
    let (status, headers, body) = get("/").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"<html>app</html>");
    assert_eq!(header(&headers, &CACHE_CONTROL), "no-cache");
    assert_eq!(header(&headers, &CONTENT_TYPE), "text/html; charset=utf-8");
}

#[tokio::test]
async fn hashed_assets_are_immutable() {
    let (status, headers, body) = get("/assets/index-abc123.js").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body, b"console.log(1)");
    assert_eq!(
        header(&headers, &CACHE_CONTROL),
        "public, max-age=31536000, immutable"
    );
    assert_eq!(
        header(&headers, &CONTENT_TYPE),
        "application/javascript; charset=utf-8"
    );
}

#[tokio::test]
async fn other_static_files_are_revalidated() {
    let (status, headers, _) = get("/favicon.svg").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(header(&headers, &CACHE_CONTROL), "no-cache");
    assert_eq!(header(&headers, &CONTENT_TYPE), "image/svg+xml");
}

#[tokio::test]
async fn unknown_app_routes_fall_back_to_the_index() {
    for path in [
        "/pages/42",
        "/settings",
        "/bootstrap",
        "/approve/abc",
        "/files/7/",
    ] {
        let (status, headers, body) = get(path).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert_eq!(body, b"<html>app</html>", "{path}");
        assert_eq!(header(&headers, &CACHE_CONTROL), "no-cache", "{path}");
    }
}

#[tokio::test]
async fn missing_files_with_an_extension_are_not_the_index() {
    for path in ["/assets/missing.js", "/robots.txt", "/a/b/c.png"] {
        assert_eq!(get(path).await.0, StatusCode::NOT_FOUND, "{path}");
    }
}

#[tokio::test]
async fn app_paths_of_files_get_the_index_even_with_an_extension() {
    for path in ["/p/contracts/acme.pdf", "/p/notes/todo.txt"] {
        let (status, _, body) = get(path).await;
        assert_eq!(status, StatusCode::OK, "{path}");
        assert_eq!(body, b"<html>app</html>");
    }
}

#[tokio::test]
async fn backend_prefixes_never_fall_back_to_the_index() {
    for path in [
        "/api",
        "/api/nope",
        "/mcp",
        "/mcp/session",
        "/oauth/authorize",
        "/.well-known/oauth-authorization-server",
        "/healthz",
    ] {
        let (status, _, body) = get(path).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{path}");
        assert!(!body.starts_with(b"<html"), "{path}");
    }
    assert_eq!(
        get("/apiary").await.0,
        StatusCode::OK,
        "only whole segments match"
    );
}

#[tokio::test]
async fn only_reads_are_served() {
    let (status, _, _) = request(&Method::POST, "/", &HeaderMap::new()).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let (status, _, _) = request(&Method::HEAD, "/", &HeaderMap::new()).await;
    assert_eq!(status, StatusCode::OK);
}

#[tokio::test]
async fn a_matching_etag_is_not_modified() {
    let (_, headers, _) = get("/").await;
    let tag = header(&headers, &ETAG).to_owned();
    assert_eq!(tag.len(), 66);
    let mut conditional = HeaderMap::new();
    conditional.insert(IF_NONE_MATCH, HeaderValue::from_str(&tag).unwrap());
    let (status, headers, body) = request(&Method::GET, "/", &conditional).await;
    assert_eq!(status, StatusCode::NOT_MODIFIED);
    assert_eq!(body, Vec::<u8>::new());
    assert_eq!(header(&headers, &ETAG), tag);
}

#[test]
fn without_a_build_nothing_is_served() {
    let response = respond(&Method::GET, "/", &HeaderMap::new(), |_| None);
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
