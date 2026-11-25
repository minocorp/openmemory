use axum::extract::{Request, State};
use axum::http::HeaderValue;
use axum::http::header::{
    CACHE_CONTROL, CONTENT_SECURITY_POLICY, REFERRER_POLICY, STRICT_TRANSPORT_SECURITY,
    X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS,
};
use axum::middleware::Next;
use axum::response::Response;

use crate::config::PublicUrl;

const CONTENT_POLICY: &str = "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self'; img-src 'self' data: blob:; font-src 'self' data:; media-src 'self'; worker-src 'self'; base-uri 'none'; form-action 'self'; frame-ancestors 'none'";
const HSTS: &str = "max-age=31536000";
const UNCACHED_PREFIXES: [&str; 4] = ["/api/auth/", "/oauth/", "/api/oauth/", "/api/upload-links/"];

#[derive(Debug, Clone, Copy)]
pub(crate) struct SecurityHeaders {
    hsts: bool,
}

impl SecurityHeaders {
    pub(crate) fn for_origin(public_url: &PublicUrl) -> Self {
        Self {
            hsts: public_url.origin().scheme() == "https",
        }
    }
}

pub(crate) async fn apply(
    State(policy): State<SecurityHeaders>,
    request: Request,
    next: Next,
) -> Response {
    let uncached = UNCACHED_PREFIXES
        .iter()
        .any(|prefix| request.uri().path().starts_with(prefix));
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(X_FRAME_OPTIONS, HeaderValue::from_static("DENY"));
    headers.insert(REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    headers.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    if !headers.contains_key(CONTENT_SECURITY_POLICY) {
        headers.insert(
            CONTENT_SECURITY_POLICY,
            HeaderValue::from_static(CONTENT_POLICY),
        );
    }
    if policy.hsts {
        headers.insert(STRICT_TRANSPORT_SECURITY, HeaderValue::from_static(HSTS));
    }
    if uncached {
        headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    }
    response
}

#[cfg(test)]
mod tests;
