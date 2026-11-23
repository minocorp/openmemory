use std::borrow::Cow;

use axum::body::Body;
use axum::http::header::{
    CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH, X_CONTENT_TYPE_OPTIONS,
};
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use memory_api::Problem;
use rust_embed::RustEmbed;

use crate::problem::respond as respond_problem;
use crate::storage::Sha256Hash;

const INDEX: &str = "index.html";
const HASHED_ASSETS: &str = "assets/";
const APP_FILE_PREFIX: &str = "p/";
const BACKEND_PREFIXES: [&str; 5] = ["/api", "/mcp", "/oauth", "/.well-known", "/healthz"];
const IMMUTABLE: &str = "public, max-age=31536000, immutable";
const REVALIDATE: &str = "no-cache";

#[derive(RustEmbed)]
#[folder = "../../web/dist"]
#[allow_missing = true]
struct Assets;

struct Asset {
    data: Cow<'static, [u8]>,
    mime: String,
    hash: [u8; 32],
}

pub(crate) async fn serve(method: Method, uri: Uri, headers: HeaderMap) -> Response {
    respond(&method, uri.path(), &headers, |name| {
        Assets::get(name).map(|file| Asset {
            data: file.data,
            mime: file.metadata.mimetype().to_owned(),
            hash: file.metadata.sha256_hash(),
        })
    })
}

fn respond(
    method: &Method,
    path: &str,
    headers: &HeaderMap,
    lookup: impl Fn(&str) -> Option<Asset>,
) -> Response {
    if !matches!(*method, Method::GET | Method::HEAD) || is_backend(path) {
        return not_found();
    }
    let name = path.trim_start_matches('/');
    let name = if name.is_empty() { INDEX } else { name };
    if let Some(asset) = lookup(name) {
        return asset_response(name, asset, headers);
    }
    if has_extension(name) && !name.starts_with(APP_FILE_PREFIX) {
        return not_found();
    }
    lookup(INDEX).map_or_else(not_found, |asset| asset_response(INDEX, asset, headers))
}

fn is_backend(path: &str) -> bool {
    BACKEND_PREFIXES.iter().any(|prefix| {
        path.strip_prefix(prefix)
            .is_some_and(|rest| rest.is_empty() || rest.starts_with('/'))
    })
}

fn has_extension(name: &str) -> bool {
    name.rsplit('/')
        .next()
        .is_some_and(|last| last.contains('.'))
}

fn asset_response(name: &str, asset: Asset, request: &HeaderMap) -> Response {
    let tag = HeaderValue::from_str(&format!(
        "\"{}\"",
        Sha256Hash::from_bytes(asset.hash).to_hex()
    ))
    .unwrap_or_else(|_| HeaderValue::from_static("\"\""));
    let cache = if name.starts_with(HASHED_ASSETS) {
        IMMUTABLE
    } else {
        REVALIDATE
    };
    let fresh = request
        .get(IF_NONE_MATCH)
        .is_some_and(|value| value.as_bytes() == tag.as_bytes());
    let mut response = if fresh {
        StatusCode::NOT_MODIFIED.into_response()
    } else {
        Response::new(Body::from(asset.data.into_owned()))
    };
    let headers = response.headers_mut();
    headers.insert(ETAG, tag);
    headers.insert(CACHE_CONTROL, HeaderValue::from_static(cache));
    headers.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    if let Ok(value) = HeaderValue::from_str(&content_type(&asset.mime)) {
        headers.insert(CONTENT_TYPE, value);
    }
    response
}

fn content_type(mime: &str) -> String {
    if mime.starts_with("text/") || mime == "application/javascript" {
        format!("{mime}; charset=utf-8")
    } else {
        mime.to_owned()
    }
}

fn not_found() -> Response {
    respond_problem(Problem::new("not_found", 404))
}

#[cfg(test)]
mod tests;
