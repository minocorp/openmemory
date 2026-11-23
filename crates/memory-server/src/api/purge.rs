use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};
use webauthn_rs::prelude::{PublicKeyCredential, RequestChallengeResponse};

use crate::api::error::ApiError;
use crate::auth::ceremony::{self, Assertion};
use crate::auth::token::Secret;
use crate::auth::{AuthError, WebSession};
use crate::files::FileId;
use crate::pages::ids::PageId;
use crate::pages::purge;
use crate::state::AppState;
use memory_api::Problem;

#[derive(Debug, Serialize, ToSchema)]
struct PurgeChallengeView {
    challenge: String,
    #[schema(value_type = Object)]
    options: RequestChallengeResponse,
}

#[derive(Debug, Deserialize, ToSchema)]
struct PurgeRequest {
    challenge: String,
    #[schema(value_type = Object)]
    credential: PublicKeyCredential,
}

#[derive(OpenApi)]
#[openapi(
    paths(purge_challenge, purge_page, purge_file),
    components(schemas(PurgeChallengeView, PurgeRequest))
)]
pub(super) struct Doc;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/purge/challenge", post(purge_challenge))
        .route("/api/pages/{id}/purge", post(purge_page))
        .route("/api/files/{id}/purge", post(purge_file))
}

async fn confirm(
    state: &AppState,
    session: &WebSession,
    request: &PurgeRequest,
) -> Result<(), ApiError> {
    session.require_admin()?;
    let asserted = ceremony::finish_authentication(
        state,
        &Secret::from_text(&request.challenge),
        &request.credential,
        Assertion::Purge,
    )
    .await?;
    if asserted == session.user_id() {
        Ok(())
    } else {
        Err(AuthError::InvalidCredentials.into())
    }
}

#[utoipa::path(
    post,
    path = "/api/purge/challenge",
    tag = "purge",
    security(("session" = [])),
    responses(
        (status = 200, body = PurgeChallengeView),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 403, body = Problem, content_type = "application/problem+json"),
    )
)]
async fn purge_challenge(
    State(state): State<AppState>,
    session: WebSession,
) -> Result<Json<PurgeChallengeView>, ApiError> {
    session.require_admin()?;
    let (challenge, options) =
        ceremony::start_authentication(&state, session.user_id(), Assertion::Purge).await?;
    Ok(Json(PurgeChallengeView {
        challenge: challenge.expose().to_owned(),
        options,
    }))
}

#[utoipa::path(
    post,
    path = "/api/pages/{id}/purge",
    tag = "purge",
    security(("session" = [])),
    params(("id" = i64, Path)),
    request_body = PurgeRequest,
    responses(
        (status = 204, description = "Every version of the page is gone; a tombstone without content remains"),
        (status = 400, body = Problem, content_type = "application/problem+json"),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 403, body = Problem, content_type = "application/problem+json"),
        (status = 404, body = Problem, content_type = "application/problem+json"),
    )
)]
async fn purge_page(
    State(state): State<AppState>,
    session: WebSession,
    Path(id): Path<PageId>,
    Json(request): Json<PurgeRequest>,
) -> Result<StatusCode, ApiError> {
    confirm(&state, &session, &request).await?;
    purge::purge(state.pool(), session.user_id(), id).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(
    post,
    path = "/api/files/{id}/purge",
    tag = "purge",
    security(("session" = [])),
    params(("id" = i64, Path)),
    request_body = PurgeRequest,
    responses(
        (status = 204, description = "The file row and its blob are gone; a tombstone without content remains"),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 403, body = Problem, content_type = "application/problem+json"),
        (status = 404, body = Problem, content_type = "application/problem+json"),
    )
)]
async fn purge_file(
    State(state): State<AppState>,
    session: WebSession,
    Path(id): Path<FileId>,
    Json(request): Json<PurgeRequest>,
) -> Result<StatusCode, ApiError> {
    confirm(&state, &session, &request).await?;
    state.files().purge(session.user_id(), id).await?;
    Ok(StatusCode::NO_CONTENT)
}
