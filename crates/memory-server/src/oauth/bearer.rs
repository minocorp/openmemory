use axum::extract::FromRequestParts;
use axum::http::header::{AUTHORIZATION, WWW_AUTHENTICATE};
use axum::http::request::Parts;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::access::{Actor, connections};
use crate::auth::token::Secret;
use crate::oauth::metadata::resource_metadata_url;
use crate::oauth::queries::tokens;
use crate::state::AppState;

#[derive(Debug, Clone)]
pub(crate) struct McpActor(pub(crate) Actor);

#[derive(Debug)]
pub(crate) enum BearerRejection {
    Missing(String),
    Invalid(String),
    Internal,
}

impl FromRequestParts<AppState> for McpActor {
    type Rejection = BearerRejection;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let metadata = resource_metadata_url(state);
        let Some(header) = parts.headers.get(AUTHORIZATION) else {
            return Err(BearerRejection::Missing(metadata));
        };
        let token = header
            .to_str()
            .ok()
            .and_then(|value| value.split_once(' '))
            .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
            .map(|(_, token)| Secret::from_text(token.trim()))
            .ok_or_else(|| BearerRejection::Invalid(metadata.clone()))?;
        let mut conn = state.pool().acquire().await.map_err(internal)?;
        let connection_id = tokens::connection_of_access(&mut conn, &token.hash())
            .await
            .map_err(internal)?
            .ok_or_else(|| BearerRejection::Invalid(metadata.clone()))?;
        let actor = connections::load_actor(&mut conn, connection_id)
            .await
            .map_err(internal)?
            .ok_or(BearerRejection::Invalid(metadata))?;
        connections::touch(&mut conn, connection_id)
            .await
            .map_err(internal)?;
        Ok(Self(actor))
    }
}

fn internal(error: impl std::error::Error) -> BearerRejection {
    tracing::error!(%error, "bearer lookup failed");
    BearerRejection::Internal
}

impl IntoResponse for BearerRejection {
    fn into_response(self) -> Response {
        let challenge = match self {
            Self::Missing(metadata) => format!(r#"Bearer resource_metadata="{metadata}""#),
            Self::Invalid(metadata) => {
                format!(r#"Bearer error="invalid_token", resource_metadata="{metadata}""#)
            }
            Self::Internal => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
        };
        let value = HeaderValue::from_str(&challenge)
            .unwrap_or_else(|_| HeaderValue::from_static("Bearer"));
        (StatusCode::UNAUTHORIZED, [(WWW_AUTHENTICATE, value)]).into_response()
    }
}

#[cfg(test)]
mod tests;
