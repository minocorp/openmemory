use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;

use crate::access::connections::{self, ConnectionError, scopes};
use crate::access::{ConnectionId, Folders, Name};
use crate::auth::confirmation::{Action, Confirmation};
use crate::auth::{AuthError, WebSession};
use crate::error::ErrorBody;
use crate::oauth::new_personal_token;
use crate::state::AppState;

const SECONDS_PER_DAY: f64 = 86_400.0;
const MAX_DAYS: u32 = 365;

#[derive(Debug, Deserialize, ToSchema)]
pub(super) struct TokenCreate {
    name: Name,
    scopes: Option<Vec<String>>,
    can_write: Option<bool>,
    expires_in_days: u32,
    confirmation: Confirmation,
}

#[derive(Debug, Serialize, ToSchema)]
pub(super) struct TokenCreated {
    id: ConnectionId,
    name: String,
    token: String,
    #[serde(with = "time::serde::rfc3339")]
    expires_at: OffsetDateTime,
}

#[utoipa::path(
    post,
    path = "/api/tokens",
    tag = "connections",
    security(("session" = [])),
    request_body = TokenCreate,
    responses(
        (status = 201, description = "The token is shown once", body = TokenCreated),
        (status = 400, body = ErrorBody),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 409, body = ErrorBody),
    )
)]
pub(super) async fn create_token(
    State(state): State<AppState>,
    session: WebSession,
    Json(request): Json<TokenCreate>,
) -> Result<(StatusCode, Json<TokenCreated>), AuthError> {
    if !(1..=MAX_DAYS).contains(&request.expires_in_days) {
        return Err(AuthError::InvalidInput("expires_in_days"));
    }
    let folders = match &request.scopes {
        Some(raw) => scopes::decode(raw).map_err(|_| AuthError::InvalidInput("scopes"))?,
        None => Folders::All,
    };
    request
        .confirmation
        .verify(&state, &session, Action::CreateToken)
        .await?;
    let token = new_personal_token();
    let mut conn = state.pool().acquire().await?;
    let (id, expires_at) = connections::create_personal(
        &mut conn,
        session.user_id(),
        &request.name,
        &folders,
        request.can_write.unwrap_or(true),
        &token.hash(),
        f64::from(request.expires_in_days) * SECONDS_PER_DAY,
    )
    .await
    .map_err(|error: ConnectionError| AuthError::from(error))?;
    Ok((
        StatusCode::CREATED,
        Json(TokenCreated {
            id,
            name: request.name.to_string(),
            token: token.expose().to_owned(),
            expires_at,
        }),
    ))
}
