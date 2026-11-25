use axum::extract::FromRequestParts;
use axum::http::header::{AUTHORIZATION, WWW_AUTHENTICATE};
use axum::http::request::Parts;
use axum::http::{HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};

use crate::access::{Actor, Audience, connections};
use crate::auth::token::Secret;
use crate::oauth::error::OAuthError;
use crate::oauth::metadata::resource_metadata_url;
use crate::oauth::queries::tokens;
use crate::state::AppState;

const PERSONAL_PREFIX: &str = "mpat_";

#[derive(Debug, Clone)]
pub(crate) struct McpActor(pub(crate) Actor);

#[derive(Debug)]
pub(crate) enum BearerRejection {
    Missing(String),
    Invalid(String),
    Internal,
}

#[derive(Debug)]
pub(crate) enum Credential {
    Absent,
    Malformed,
    Token(Secret),
}

pub(crate) fn credential(parts: &Parts) -> Credential {
    let Some(header) = parts.headers.get(AUTHORIZATION) else {
        return Credential::Absent;
    };
    header
        .to_str()
        .ok()
        .and_then(|value| value.split_once(' '))
        .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("bearer"))
        .map_or(Credential::Malformed, |(_, token)| {
            Credential::Token(Secret::from_text(token.trim()))
        })
}

pub(crate) fn new_personal_token() -> Secret {
    Secret::from_text(&format!("{PERSONAL_PREFIX}{}", Secret::generate().expose()))
}

pub(crate) async fn resolve(
    state: &AppState,
    token: &Secret,
    audience: Audience,
) -> Result<Option<Actor>, OAuthError> {
    let mut conn = state.pool().acquire().await?;
    let hash = token.hash();
    let personal = audience == Audience::Api && token.expose().starts_with(PERSONAL_PREFIX);
    let connection_id = if personal {
        connections::find_by_personal_token(&mut conn, &hash).await?
    } else {
        tokens::connection_of_access(&mut conn, &hash).await?
    };
    let Some(connection_id) = connection_id else {
        return Ok(None);
    };
    let actor = connections::load_actor_in(&mut conn, connection_id, Some(audience)).await?;
    if actor.is_some() {
        connections::touch(&mut conn, connection_id).await?;
    }
    Ok(actor)
}

impl FromRequestParts<AppState> for McpActor {
    type Rejection = BearerRejection;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let metadata = resource_metadata_url(state, Audience::Mcp);
        let token = match credential(parts) {
            Credential::Absent => return Err(BearerRejection::Missing(metadata)),
            Credential::Malformed => return Err(BearerRejection::Invalid(metadata)),
            Credential::Token(token) => token,
        };
        match resolve(state, &token, Audience::Mcp).await {
            Ok(Some(actor)) => Ok(Self(actor)),
            Ok(None) => Err(BearerRejection::Invalid(metadata)),
            Err(error) => {
                tracing::error!(%error, "bearer lookup failed");
                Err(BearerRejection::Internal)
            }
        }
    }
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
