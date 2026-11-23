use axum::extract::{Path, State};
use axum::http::header::SET_COOKIE;
use axum::http::{HeaderName, HeaderValue, StatusCode};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::{OpenApi, ToSchema};
use webauthn_rs::prelude::{CreationChallengeResponse, RegisterPublicKeyCredential};

use crate::access::{DisplayName, Name, UserId, Username};
use crate::auth::ceremony::{self, Purpose};
use crate::auth::confirmation::{Action, Confirmation, Confirmed};
use crate::auth::ids::PasskeyId;
use crate::auth::queries::{passkeys, users};
use crate::auth::token::Secret;
use crate::auth::users::profile_conflict;
use crate::auth::{AuthError, WebSession, session};
use crate::state::AppState;
use memory_api::Problem;

#[derive(Debug, Serialize, ToSchema)]
struct Me {
    id: UserId,
    username: String,
    display_name: String,
    is_admin: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
struct MeUpdate {
    username: Option<Username>,
    display_name: Option<DisplayName>,
}

#[derive(Debug, Serialize, ToSchema)]
struct PasskeyView {
    id: PasskeyId,
    name: String,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    last_used_at: Option<OffsetDateTime>,
}

#[derive(Debug, Deserialize, ToSchema)]
struct AddPasskeyStart {
    name: Name,
    confirmation: Confirmation,
}

#[derive(Debug, Serialize, ToSchema)]
struct AddPasskeyStarted {
    challenge: String,
    #[schema(value_type = Object)]
    options: CreationChallengeResponse,
}

#[derive(Debug, Deserialize, ToSchema)]
struct AddPasskeyFinish {
    challenge: String,
    #[schema(value_type = Object)]
    credential: RegisterPublicKeyCredential,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        me,
        update_me,
        logout,
        list_passkeys,
        start_add_passkey,
        finish_add_passkey,
        delete_passkey
    ),
    components(schemas(
        Me,
        MeUpdate,
        PasskeyView,
        AddPasskeyStart,
        AddPasskeyStarted,
        AddPasskeyFinish
    ))
)]
pub(super) struct Doc;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/me", get(me).patch(update_me))
        .route("/api/auth/logout", post(logout))
        .route("/api/auth/passkeys", get(list_passkeys))
        .route("/api/auth/passkeys/start", post(start_add_passkey))
        .route("/api/auth/passkeys/finish", post(finish_add_passkey))
        .route("/api/auth/passkeys/{id}", delete(delete_passkey))
}

#[utoipa::path(
    get,
    path = "/api/auth/me",
    tag = "auth",
    security(("session" = [])),
    responses((status = 200, body = Me), (status = 401, body = Problem, content_type = "application/problem+json"))
)]
async fn me(session: WebSession) -> Json<Me> {
    Json(Me {
        id: session.user_id(),
        username: session.actor.name.clone(),
        display_name: session.display_name.clone(),
        is_admin: session.is_admin,
    })
}

#[utoipa::path(
    patch,
    path = "/api/auth/me",
    tag = "auth",
    security(("session" = [])),
    request_body = MeUpdate,
    responses(
        (status = 200, body = Me),
        (status = 400, body = Problem, content_type = "application/problem+json"),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "The username is taken", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn update_me(
    State(state): State<AppState>,
    session: WebSession,
    Json(update): Json<MeUpdate>,
) -> Result<Json<Me>, AuthError> {
    if update.username.is_none() && update.display_name.is_none() {
        return Err(AuthError::InvalidInput("update"));
    }
    let mut conn = state.pool().acquire().await?;
    let row = users::update_profile(
        &mut conn,
        session.user_id(),
        update.username.as_ref().map(Username::as_str),
        update.display_name.as_ref().map(DisplayName::as_str),
    )
    .await
    .map_err(profile_conflict)?
    .ok_or(AuthError::Unauthenticated)?;
    Ok(Json(Me {
        id: row.id,
        username: row.username,
        display_name: row.display_name,
        is_admin: row.is_admin,
    }))
}

#[utoipa::path(
    post,
    path = "/api/auth/logout",
    tag = "auth",
    security(("session" = [])),
    responses((status = 204), (status = 401, body = Problem, content_type = "application/problem+json"))
)]
async fn logout(
    State(state): State<AppState>,
    session: WebSession,
) -> Result<([(HeaderName, HeaderValue); 1], StatusCode), AuthError> {
    let mut conn = state.pool().acquire().await?;
    session::end(&mut conn, &session).await?;
    Ok((
        [(SET_COOKIE, session::clear_cookie())],
        StatusCode::NO_CONTENT,
    ))
}

#[utoipa::path(
    get,
    path = "/api/auth/passkeys",
    tag = "auth",
    security(("session" = [])),
    responses((status = 200, body = Vec<PasskeyView>), (status = 401, body = Problem, content_type = "application/problem+json"))
)]
async fn list_passkeys(
    State(state): State<AppState>,
    session: WebSession,
) -> Result<Json<Vec<PasskeyView>>, AuthError> {
    let mut conn = state.pool().acquire().await?;
    let rows = passkeys::list(&mut conn, session.user_id()).await?;
    Ok(Json(
        rows.into_iter()
            .map(|row| PasskeyView {
                id: row.id,
                name: row.name,
                created_at: row.created_at,
                last_used_at: row.last_used_at,
            })
            .collect(),
    ))
}

#[utoipa::path(
    post,
    path = "/api/auth/passkeys/start",
    tag = "auth",
    security(("session" = [])),
    request_body = AddPasskeyStart,
    responses((status = 200, body = AddPasskeyStarted), (status = 401, body = Problem, content_type = "application/problem+json"))
)]
async fn start_add_passkey(
    State(state): State<AppState>,
    session: WebSession,
    Json(request): Json<AddPasskeyStart>,
) -> Result<Json<AddPasskeyStarted>, AuthError> {
    request
        .confirmation
        .verify(&state, &session, Action::AddPasskey)
        .await?;
    let mut conn = state.pool().acquire().await?;
    let account = ceremony::account_of(&mut conn, session.user_id()).await?;
    let existing = ceremony::load_passkeys(&mut conn, session.user_id()).await?;
    let (challenge, options) = ceremony::start_registration(
        &state,
        ceremony::Account {
            user_handle: ceremony::user_handle(&account.user_handle)?,
            username: &account.username,
            display_name: &account.display_name,
        },
        existing
            .iter()
            .map(|passkey| passkey.cred_id().clone())
            .collect(),
        Purpose::AddPasskey {
            user_id: session.user_id(),
        },
        request.name.to_string(),
    )
    .await?;
    Ok(Json(AddPasskeyStarted {
        challenge: challenge.expose().to_owned(),
        options,
    }))
}

#[utoipa::path(
    post,
    path = "/api/auth/passkeys/finish",
    tag = "auth",
    security(("session" = [])),
    request_body = AddPasskeyFinish,
    responses(
        (status = 201),
        (status = 400, body = Problem, content_type = "application/problem+json"),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 409, body = Problem, content_type = "application/problem+json"),
    )
)]
async fn finish_add_passkey(
    State(state): State<AppState>,
    session: WebSession,
    Json(request): Json<AddPasskeyFinish>,
) -> Result<StatusCode, AuthError> {
    let finished = ceremony::finish_registration(
        &state,
        &Secret::from_text(&request.challenge),
        &request.credential,
    )
    .await?;
    match finished.purpose {
        Purpose::AddPasskey { user_id } if user_id == session.user_id() => {}
        Purpose::AddPasskey { .. }
        | Purpose::Bootstrap { .. }
        | Purpose::Invite { .. }
        | Purpose::Recovery { .. } => return Err(AuthError::InvalidToken),
    }
    let mut conn = state.pool().acquire().await?;
    ceremony::store_passkey(
        &mut conn,
        session.user_id(),
        &finished.passkey,
        &finished.passkey_name,
    )
    .await?;
    Ok(StatusCode::CREATED)
}

#[utoipa::path(
    delete,
    path = "/api/auth/passkeys/{id}",
    tag = "auth",
    security(("session" = [])),
    params(("id" = i64, Path)),
    request_body = Confirmed,
    responses(
        (status = 204),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 404, body = Problem, content_type = "application/problem+json"),
        (status = 409, body = Problem, content_type = "application/problem+json"),
    )
)]
async fn delete_passkey(
    State(state): State<AppState>,
    session: WebSession,
    Path(id): Path<PasskeyId>,
    Json(request): Json<Confirmed>,
) -> Result<StatusCode, AuthError> {
    request
        .confirmation
        .verify(&state, &session, Action::RemovePasskey)
        .await?;
    let mut tx = state.pool().begin().await?;
    let held = passkeys::lock_for_user(&mut tx, session.user_id()).await?;
    if !held.contains(&id) {
        return Err(AuthError::NotFound);
    }
    if held.len() <= 1 {
        return Err(AuthError::LastPasskey);
    }
    if !passkeys::delete(&mut tx, session.user_id(), id).await? {
        return Err(AuthError::NotFound);
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
