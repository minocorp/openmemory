use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::{delete, get};
use axum::{Json, Router};
use serde::Serialize;
use time::OffsetDateTime;
use utoipa::{OpenApi, ToSchema};

use crate::auth::confirmation::{Action, Confirmed};
use crate::auth::ids::InviteId;
use crate::auth::queries::invites::{self, InviteRow};
use crate::auth::token::Secret;
use crate::auth::{AuthError, WebSession};
use crate::error::ErrorBody;
use crate::state::AppState;

const INVITE_SECONDS: f64 = 7.0 * 24.0 * 3600.0;

#[derive(Debug, Serialize, ToSchema)]
struct CreatedInvite {
    id: InviteId,
    token: String,
}

#[derive(Debug, Serialize, ToSchema)]
struct InviteView {
    id: InviteId,
    inviter: String,
    inviter_display_name: String,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339")]
    expires_at: OffsetDateTime,
}

#[derive(OpenApi)]
#[openapi(
    paths(list_invites, create_invite, delete_invite),
    components(schemas(CreatedInvite, InviteView))
)]
pub(super) struct Doc;

impl From<InviteRow> for InviteView {
    fn from(row: InviteRow) -> Self {
        Self {
            id: row.id,
            inviter: row.inviter,
            inviter_display_name: row.inviter_display_name,
            created_at: row.created_at,
            expires_at: row.expires_at,
        }
    }
}

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/invites", get(list_invites).post(create_invite))
        .route("/api/invites/{id}", delete(delete_invite))
}

#[utoipa::path(
    post,
    path = "/api/invites",
    tag = "invites",
    security(("session" = [])),
    request_body = Confirmed,
    responses(
        (status = 201, body = CreatedInvite),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
    )
)]
async fn create_invite(
    State(state): State<AppState>,
    session: WebSession,
    Json(request): Json<Confirmed>,
) -> Result<(StatusCode, Json<CreatedInvite>), AuthError> {
    session.require_admin()?;
    request
        .confirmation
        .verify(&state, &session, Action::CreateInvite)
        .await?;
    let token = Secret::generate();
    let mut conn = state.pool().acquire().await?;
    let id = invites::insert(&mut conn, session.user_id(), &token.hash(), INVITE_SECONDS).await?;
    Ok((
        StatusCode::CREATED,
        Json(CreatedInvite {
            id,
            token: token.expose().to_owned(),
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/api/invites",
    tag = "invites",
    security(("session" = [])),
    responses(
        (status = 200, body = Vec<InviteView>),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
    )
)]
async fn list_invites(
    State(state): State<AppState>,
    session: WebSession,
) -> Result<Json<Vec<InviteView>>, AuthError> {
    session.require_admin()?;
    let mut conn = state.pool().acquire().await?;
    let rows = invites::list_live(&mut conn).await?;
    Ok(Json(rows.into_iter().map(InviteView::from).collect()))
}

#[utoipa::path(
    delete,
    path = "/api/invites/{id}",
    tag = "invites",
    security(("session" = [])),
    params(("id" = i64, Path)),
    responses(
        (status = 204),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 404, body = ErrorBody),
    )
)]
async fn delete_invite(
    State(state): State<AppState>,
    session: WebSession,
    Path(id): Path<InviteId>,
) -> Result<StatusCode, AuthError> {
    session.require_admin()?;
    let mut conn = state.pool().acquire().await?;
    if invites::delete(&mut conn, id).await? {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(AuthError::NotFound)
    }
}
