use axum::extract::FromRequestParts;
use axum::http::header::WWW_AUTHENTICATE;
use axum::http::request::Parts;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::access::{Actor, Audience};
use crate::auth::{AuthError, csrf, session};
use crate::oauth::{Credential, OAuthError, credential, resolve, resource_metadata_url};
use crate::state::AppState;

#[derive(Debug, Clone)]
pub(crate) struct ApiSession {
    pub(crate) actor: Actor,
}

#[derive(Debug, Clone)]
pub(crate) struct BinaryApiSession(pub(crate) ApiSession);

#[derive(Debug)]
pub(crate) struct ApiRejection {
    error: AuthError,
    challenge: Option<String>,
}

#[derive(Debug, Clone, Copy)]
enum CookieCheck {
    Full,
    OriginOnly,
}

impl FromRequestParts<AppState> for ApiSession {
    type Rejection = ApiRejection;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        authenticate(parts, state, CookieCheck::Full).await
    }
}

impl FromRequestParts<AppState> for BinaryApiSession {
    type Rejection = ApiRejection;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        authenticate(parts, state, CookieCheck::OriginOnly)
            .await
            .map(Self)
    }
}

async fn authenticate(
    parts: &Parts,
    state: &AppState,
    check: CookieCheck,
) -> Result<ApiSession, ApiRejection> {
    let metadata = resource_metadata_url(state, Audience::Api);
    match credential(parts) {
        Credential::Token(token) => match resolve(state, &token, Audience::Api).await {
            Ok(Some(actor)) => Ok(ApiSession { actor }),
            Ok(None) => Err(ApiRejection::invalid_bearer(&metadata)),
            Err(error) => Err(internal(error)),
        },
        Credential::Malformed => Err(ApiRejection::invalid_bearer(&metadata)),
        Credential::Absent => {
            let issuer = state.public_url().issuer();
            match check {
                CookieCheck::Full => csrf::check(parts, issuer),
                CookieCheck::OriginOnly => csrf::check_origin(parts, issuer),
            }
            .map_err(ApiRejection::plain)?;
            match session::authenticate(parts, state).await {
                Ok(web) => Ok(ApiSession { actor: web.actor }),
                Err(AuthError::Unauthenticated) => Err(ApiRejection {
                    error: AuthError::Unauthenticated,
                    challenge: Some(format!(r#"Bearer resource_metadata="{metadata}""#)),
                }),
                Err(error) => Err(ApiRejection::plain(error)),
            }
        }
    }
}

fn internal(error: OAuthError) -> ApiRejection {
    ApiRejection::plain(match error {
        OAuthError::Database(error) => AuthError::Database(error),
        OAuthError::Connection(error) => error.into(),
        OAuthError::Code(_) => AuthError::InvalidBearer,
    })
}

impl ApiRejection {
    fn plain(error: AuthError) -> Self {
        Self {
            error,
            challenge: None,
        }
    }

    fn invalid_bearer(metadata: &str) -> Self {
        Self {
            error: AuthError::InvalidBearer,
            challenge: Some(format!(
                r#"Bearer error="invalid_token", resource_metadata="{metadata}""#
            )),
        }
    }
}

impl IntoResponse for ApiRejection {
    fn into_response(self) -> Response {
        let mut response = self.error.into_response();
        if response.status() == StatusCode::UNAUTHORIZED
            && let Some(value) = self
                .challenge
                .and_then(|challenge| HeaderValue::from_str(&challenge).ok())
        {
            response.headers_mut().insert(WWW_AUTHENTICATE, value);
        }
        response
    }
}
