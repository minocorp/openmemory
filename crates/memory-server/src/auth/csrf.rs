use axum::extract::FromRequestParts;
use axum::http::Method;
use axum::http::header::{CONTENT_TYPE, ORIGIN};
use axum::http::request::Parts;

use crate::auth::AuthError;
use crate::state::AppState;

#[derive(Debug, Clone, Copy)]
pub(crate) struct SameOrigin;

impl FromRequestParts<AppState> for SameOrigin {
    type Rejection = AuthError;

    fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> {
        std::future::ready(check(parts, state.public_url().issuer()).map(|()| Self))
    }
}

pub(crate) fn check(parts: &Parts, expected_origin: &str) -> Result<(), AuthError> {
    check_origin(parts, expected_origin)?;
    if is_safe(&parts.method) {
        return Ok(());
    }
    let is_json = parts
        .headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|essence| essence.trim().eq_ignore_ascii_case("application/json"));
    if is_json {
        Ok(())
    } else {
        Err(AuthError::UnsupportedMediaType)
    }
}

pub(crate) fn check_origin(parts: &Parts, expected_origin: &str) -> Result<(), AuthError> {
    if is_safe(&parts.method) {
        return Ok(());
    }
    let origin = parts
        .headers
        .get(ORIGIN)
        .and_then(|value| value.to_str().ok());
    if origin == Some(expected_origin) {
        Ok(())
    } else {
        Err(AuthError::OriginMismatch)
    }
}

fn is_safe(method: &Method) -> bool {
    matches!(*method, Method::GET | Method::HEAD | Method::OPTIONS)
}
