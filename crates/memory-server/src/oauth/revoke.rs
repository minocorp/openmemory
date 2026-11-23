use axum::Form;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Deserialize;

use crate::access::connections;
use crate::auth::token::Secret;
use crate::oauth::clients;
use crate::oauth::error::{OAuthError, OAuthErrorCode};
use crate::oauth::queries::tokens;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub(super) struct RevokeRequest {
    token: Option<String>,
    client_id: Option<String>,
}

pub(super) async fn revoke(
    State(state): State<AppState>,
    Form(request): Form<RevokeRequest>,
) -> Result<StatusCode, OAuthError> {
    let (Some(token), Some(client_id)) = (request.token.as_deref(), request.client_id.as_deref())
    else {
        return Err(OAuthErrorCode::InvalidRequest.into());
    };
    let mut conn = state.pool().acquire().await?;
    let client = clients::stored(&mut conn, client_id).await?;
    let owner = tokens::owner_of_token(&mut conn, &Secret::from_text(token).hash()).await?;
    // RFC 7009 section 2.2: unknown tokens and other clients' tokens still answer 200.
    if let Some(owner) = owner.filter(|owner| owner.oauth_client_id == client.id) {
        connections::revoke_one(&mut conn, owner.connection_id).await?;
    }
    Ok(StatusCode::OK)
}
