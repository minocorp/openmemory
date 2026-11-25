use axum::extract::{Path, State};
use axum::routing::{get, patch};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::{OpenApi, ToSchema};

use crate::access::{DisplayName, UserId, Username};
use crate::auth::confirmation::{Action, Confirmation};
use crate::auth::queries::users::{self, UserSummary};
use crate::auth::{AuthError, WebSession, violated_constraint};
use crate::state::AppState;
use openmemory_api::Problem;

#[derive(Debug, Serialize, ToSchema)]
struct UserView {
    id: UserId,
    username: String,
    display_name: String,
    is_admin: bool,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
}

#[derive(Debug, Deserialize, ToSchema)]
struct UserUpdate {
    is_admin: bool,
    confirmation: Confirmation,
}

#[derive(Debug, Deserialize, ToSchema)]
struct ProfileUpdate {
    username: Option<Username>,
    display_name: Option<DisplayName>,
}

#[derive(OpenApi)]
#[openapi(
    paths(list_users, update_user, update_profile),
    components(schemas(UserView, UserUpdate, ProfileUpdate))
)]
pub(super) struct Doc;

impl From<UserSummary> for UserView {
    fn from(row: UserSummary) -> Self {
        Self {
            id: row.id,
            username: row.username,
            display_name: row.display_name,
            is_admin: row.is_admin,
            created_at: row.created_at,
        }
    }
}

pub(super) fn profile_conflict(error: sqlx::Error) -> AuthError {
    if violated_constraint(&error) == Some("users_username_key") {
        AuthError::NameTaken
    } else {
        AuthError::Database(error)
    }
}

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/users", get(list_users))
        .route("/api/users/{id}", patch(update_user))
        .route("/api/users/{id}/profile", patch(update_profile))
}

#[utoipa::path(
    get,
    path = "/api/users",
    tag = "users",
    security(("session" = [])),
    responses(
        (status = 200, body = Vec<UserView>),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 403, body = Problem, content_type = "application/problem+json"),
    )
)]
async fn list_users(
    State(state): State<AppState>,
    session: WebSession,
) -> Result<Json<Vec<UserView>>, AuthError> {
    session.require_admin()?;
    let mut conn = state.pool().acquire().await?;
    let rows = users::list(&mut conn).await?;
    Ok(Json(rows.into_iter().map(UserView::from).collect()))
}

#[utoipa::path(
    patch,
    path = "/api/users/{id}",
    tag = "users",
    security(("session" = [])),
    params(("id" = i64, Path)),
    request_body = UserUpdate,
    responses(
        (status = 200, body = UserView),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 403, body = Problem, content_type = "application/problem+json"),
        (status = 404, body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "The user is the last admin", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn update_user(
    State(state): State<AppState>,
    session: WebSession,
    Path(id): Path<UserId>,
    Json(update): Json<UserUpdate>,
) -> Result<Json<UserView>, AuthError> {
    session.require_admin()?;
    update
        .confirmation
        .verify(&state, &session, Action::SetAdmin)
        .await?;
    let mut tx = state.pool().begin().await?;
    let admins = users::lock_admins(&mut tx).await?;
    if !update.is_admin && admins == [id] {
        return Err(AuthError::LastAdmin);
    }
    let row = users::set_admin(&mut tx, id, update.is_admin)
        .await?
        .ok_or(AuthError::NotFound)?;
    tx.commit().await?;
    Ok(Json(row.into()))
}

#[utoipa::path(
    patch,
    path = "/api/users/{id}/profile",
    tag = "users",
    security(("session" = [])),
    params(("id" = i64, Path)),
    request_body = ProfileUpdate,
    responses(
        (status = 200, body = UserView),
        (status = 400, body = Problem, content_type = "application/problem+json"),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 403, body = Problem, content_type = "application/problem+json"),
        (status = 404, body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "The username is taken", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn update_profile(
    State(state): State<AppState>,
    session: WebSession,
    Path(id): Path<UserId>,
    Json(update): Json<ProfileUpdate>,
) -> Result<Json<UserView>, AuthError> {
    session.require_admin()?;
    if update.username.is_none() && update.display_name.is_none() {
        return Err(AuthError::InvalidInput("update"));
    }
    let mut conn = state.pool().acquire().await?;
    let row = users::update_profile(
        &mut conn,
        id,
        update.username.as_ref().map(Username::as_str),
        update.display_name.as_ref().map(DisplayName::as_str),
    )
    .await
    .map_err(profile_conflict)?
    .ok_or(AuthError::NotFound)?;
    Ok(Json(row.into()))
}
