use axum::Json;
use axum::http::StatusCode;
use axum::http::header::{CACHE_CONTROL, PRAGMA};
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OAuthErrorCode {
    InvalidRequest,
    InvalidClient,
    InvalidGrant,
    UnsupportedGrantType,
    InvalidTarget,
    InvalidRedirectUri,
    InvalidClientMetadata,
    AuthorizationPending,
    SlowDown,
    AccessDenied,
    ExpiredToken,
    TemporarilyUnavailable,
    ServerError,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum OAuthError {
    #[error("oauth error")]
    Code(OAuthErrorCode),
    #[error("database failure")]
    Database(#[from] sqlx::Error),
    #[error("connection scopes are unreadable")]
    Connection(#[from] crate::access::connections::ConnectionError),
}

impl OAuthErrorCode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::InvalidRequest => "invalid_request",
            Self::InvalidClient => "invalid_client",
            Self::InvalidGrant => "invalid_grant",
            Self::UnsupportedGrantType => "unsupported_grant_type",
            Self::InvalidTarget => "invalid_target",
            Self::InvalidRedirectUri => "invalid_redirect_uri",
            Self::InvalidClientMetadata => "invalid_client_metadata",
            Self::AuthorizationPending => "authorization_pending",
            Self::SlowDown => "slow_down",
            Self::AccessDenied => "access_denied",
            Self::ExpiredToken => "expired_token",
            Self::TemporarilyUnavailable => "temporarily_unavailable",
            Self::ServerError => "server_error",
        }
    }

    fn status(self) -> StatusCode {
        match self {
            Self::InvalidClient => StatusCode::UNAUTHORIZED,
            Self::TemporarilyUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::ServerError => StatusCode::INTERNAL_SERVER_ERROR,
            Self::InvalidRequest
            | Self::InvalidGrant
            | Self::UnsupportedGrantType
            | Self::InvalidTarget
            | Self::InvalidRedirectUri
            | Self::InvalidClientMetadata
            | Self::AuthorizationPending
            | Self::SlowDown
            | Self::AccessDenied
            | Self::ExpiredToken => StatusCode::BAD_REQUEST,
        }
    }
}

impl From<OAuthErrorCode> for OAuthError {
    fn from(code: OAuthErrorCode) -> Self {
        Self::Code(code)
    }
}

impl IntoResponse for OAuthError {
    fn into_response(self) -> Response {
        let code = match &self {
            Self::Code(code) => *code,
            Self::Database(error) => {
                tracing::error!(%error, "database failure");
                OAuthErrorCode::ServerError
            }
            Self::Connection(error) => {
                tracing::error!(%error, "connection scopes are unreadable");
                OAuthErrorCode::ServerError
            }
        };
        (
            code.status(),
            [(CACHE_CONTROL, "no-store"), (PRAGMA, "no-cache")],
            Json(json!({ "error": code.as_str() })),
        )
            .into_response()
    }
}
