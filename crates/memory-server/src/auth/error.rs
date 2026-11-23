use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use webauthn_rs::prelude::WebauthnError;

#[derive(Debug, thiserror::Error)]
pub(crate) enum AuthError {
    #[error("authentication required")]
    Unauthenticated,
    #[error("invalid credentials")]
    InvalidCredentials,
    #[error("admin required")]
    Forbidden,
    #[error("not found")]
    NotFound,
    #[error("invalid or expired token")]
    InvalidToken,
    #[error("invalid input")]
    InvalidInput(&'static str),
    #[error("name is taken")]
    NameTaken,
    #[error("passkey is already registered")]
    PasskeyTaken,
    #[error("user is the last admin")]
    LastAdmin,
    #[error("passkey is the last one")]
    LastPasskey,
    #[error("passkey ceremony rejected")]
    PasskeyRejected(#[source] WebauthnError),
    #[error("origin does not match")]
    OriginMismatch,
    #[error("json content type required")]
    UnsupportedMediaType,
    #[error("webauthn failure")]
    Webauthn(#[source] WebauthnError),
    #[error("stored state is unreadable")]
    State(#[from] serde_json::Error),
    #[error("database failure")]
    Database(#[from] sqlx::Error),
}

impl AuthError {
    fn status(&self) -> StatusCode {
        match self {
            Self::Unauthenticated | Self::InvalidCredentials => StatusCode::UNAUTHORIZED,
            Self::Forbidden | Self::OriginMismatch => StatusCode::FORBIDDEN,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::InvalidToken | Self::InvalidInput(_) | Self::PasskeyRejected(_) => {
                StatusCode::BAD_REQUEST
            }
            Self::NameTaken | Self::PasskeyTaken | Self::LastPasskey | Self::LastAdmin => {
                StatusCode::CONFLICT
            }
            Self::UnsupportedMediaType => StatusCode::UNSUPPORTED_MEDIA_TYPE,
            Self::Webauthn(_) | Self::State(_) | Self::Database(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
        }
    }

    fn code(&self) -> &'static str {
        match self {
            Self::Unauthenticated => "unauthenticated",
            Self::InvalidCredentials => "invalid_credentials",
            Self::Forbidden => "forbidden",
            Self::NotFound => "not_found",
            Self::InvalidToken => "invalid_token",
            Self::InvalidInput(_) => "invalid_input",
            Self::NameTaken => "name_taken",
            Self::PasskeyTaken => "passkey_taken",
            Self::LastPasskey => "last_passkey",
            Self::LastAdmin => "last_admin",
            Self::PasskeyRejected(_) => "passkey_rejected",
            Self::OriginMismatch => "origin_mismatch",
            Self::UnsupportedMediaType => "unsupported_media_type",
            Self::Webauthn(_) | Self::State(_) | Self::Database(_) => "internal",
        }
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let status = self.status();
        match &self {
            Self::Webauthn(error) => tracing::error!(%error, "webauthn failure"),
            Self::State(error) => tracing::error!(%error, "stored state is unreadable"),
            Self::Database(error) => tracing::error!(%error, "database failure"),
            Self::PasskeyRejected(error) => tracing::warn!(%error, "passkey ceremony rejected"),
            Self::Unauthenticated
            | Self::InvalidCredentials
            | Self::Forbidden
            | Self::NotFound
            | Self::InvalidToken
            | Self::InvalidInput(_)
            | Self::NameTaken
            | Self::PasskeyTaken
            | Self::LastPasskey
            | Self::LastAdmin
            | Self::OriginMismatch
            | Self::UnsupportedMediaType => {}
        }
        let body = match &self {
            Self::InvalidInput(field) => json!({ "error": self.code(), "field": field }),
            _ => json!({ "error": self.code() }),
        };
        (status, Json(body)).into_response()
    }
}

pub(crate) fn violated_constraint(error: &sqlx::Error) -> Option<&str> {
    error
        .as_database_error()
        .filter(|database| database.is_unique_violation())
        .and_then(|database| database.constraint())
}
