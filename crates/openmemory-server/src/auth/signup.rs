use axum::extract::State;
use axum::http::header::SET_COOKIE;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use utoipa::{OpenApi, ToSchema};
use webauthn_rs::prelude::{CreationChallengeResponse, RegisterPublicKeyCredential, Uuid};

use crate::access::{DisplayName, UserId, Username};
use crate::auth::ceremony::{self, FinishedRegistration, Purpose};
use crate::auth::queries::{invites, users};
use crate::auth::token::Secret;
use crate::auth::{AuthError, SameOrigin, recovery, session, violated_constraint};
use crate::state::AppState;
use openmemory_api::Problem;

const PASSKEY_NAME: &str = "passkey";

#[derive(Debug, Deserialize, ToSchema)]
struct SignupStart {
    token: String,
    username: Username,
    display_name: DisplayName,
}

#[derive(Debug, Serialize, ToSchema)]
struct SignupStarted {
    challenge: String,
    #[schema(value_type = Object)]
    options: CreationChallengeResponse,
}

#[derive(Debug, Deserialize, ToSchema)]
struct SignupFinish {
    challenge: String,
    #[schema(value_type = Object)]
    credential: RegisterPublicKeyCredential,
}

#[derive(Debug, Serialize, ToSchema)]
struct SignedUp {
    user_id: UserId,
    recovery_code: String,
}

type FinishReply = (
    [(axum::http::HeaderName, axum::http::HeaderValue); 1],
    Json<SignedUp>,
);

#[derive(OpenApi)]
#[openapi(
    paths(start_bootstrap, finish_bootstrap, start_invite, finish_register),
    components(schemas(SignupStart, SignupStarted, SignupFinish, SignedUp))
)]
pub(super) struct Doc;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/bootstrap/start", post(start_bootstrap))
        .route("/api/auth/bootstrap/finish", post(finish_bootstrap))
        .route("/api/auth/register/start", post(start_invite))
        .route("/api/auth/register/finish", post(finish_register))
}

#[utoipa::path(
    post,
    path = "/api/auth/bootstrap/start",
    tag = "auth",
    request_body = SignupStart,
    responses(
        (status = 200, body = SignupStarted),
        (status = 400, body = Problem, content_type = "application/problem+json"),
        (status = 409, body = Problem, content_type = "application/problem+json"),
    )
)]
async fn start_bootstrap(
    State(state): State<AppState>,
    _origin: SameOrigin,
    Json(request): Json<SignupStart>,
) -> Result<Json<SignupStarted>, AuthError> {
    let mut conn = state.pool().acquire().await?;
    if !state
        .bootstrap()
        .accepts(&Secret::from_text(&request.token))
        || users::any_exists(&mut conn).await?
    {
        return Err(AuthError::InvalidToken);
    }
    let purpose = Purpose::Bootstrap {
        username: request.username.to_string(),
        display_name: request.display_name.to_string(),
        user_handle: Uuid::new_v4(),
    };
    start(&state, &mut conn, purpose).await
}

#[utoipa::path(
    post,
    path = "/api/auth/register/start",
    tag = "auth",
    request_body = SignupStart,
    responses(
        (status = 200, body = SignupStarted),
        (status = 400, body = Problem, content_type = "application/problem+json"),
        (status = 409, body = Problem, content_type = "application/problem+json"),
    )
)]
async fn start_invite(
    State(state): State<AppState>,
    _origin: SameOrigin,
    Json(request): Json<SignupStart>,
) -> Result<Json<SignupStarted>, AuthError> {
    let mut conn = state.pool().acquire().await?;
    let token = Secret::from_text(&request.token);
    let invite_id = invites::find_live(&mut conn, &token.hash())
        .await?
        .ok_or(AuthError::InvalidToken)?;
    let purpose = Purpose::Invite {
        invite_id,
        username: request.username.to_string(),
        display_name: request.display_name.to_string(),
        user_handle: Uuid::new_v4(),
    };
    start(&state, &mut conn, purpose).await
}

async fn start(
    state: &AppState,
    conn: &mut PgConnection,
    purpose: Purpose,
) -> Result<Json<SignupStarted>, AuthError> {
    let (username, display_name, user_handle) = match &purpose {
        Purpose::Bootstrap {
            username,
            display_name,
            user_handle,
        }
        | Purpose::Invite {
            username,
            display_name,
            user_handle,
            ..
        } => (username, display_name, *user_handle),
        Purpose::AddPasskey { .. } | Purpose::Recovery { .. } => {
            return Err(AuthError::InvalidToken);
        }
    };
    if users::find_by_username(conn, username).await?.is_some() {
        return Err(AuthError::NameTaken);
    }
    let account = ceremony::Account {
        user_handle,
        username,
        display_name,
    };
    let (challenge, options) = ceremony::start_registration(
        state,
        account,
        Vec::new(),
        purpose.clone(),
        PASSKEY_NAME.to_owned(),
    )
    .await?;
    Ok(Json(SignupStarted {
        challenge: challenge.expose().to_owned(),
        options,
    }))
}

#[utoipa::path(
    post,
    path = "/api/auth/bootstrap/finish",
    tag = "auth",
    request_body = SignupFinish,
    responses(
        (status = 200, body = SignedUp, headers(("Set-Cookie" = String))),
        (status = 400, body = Problem, content_type = "application/problem+json"),
        (status = 409, body = Problem, content_type = "application/problem+json"),
    )
)]
async fn finish_bootstrap(
    state: State<AppState>,
    origin: SameOrigin,
    request: Json<SignupFinish>,
) -> Result<FinishReply, AuthError> {
    finish(state, origin, request).await
}

#[utoipa::path(
    post,
    path = "/api/auth/register/finish",
    tag = "auth",
    request_body = SignupFinish,
    responses(
        (status = 200, body = SignedUp, headers(("Set-Cookie" = String))),
        (status = 400, body = Problem, content_type = "application/problem+json"),
        (status = 409, body = Problem, content_type = "application/problem+json"),
    )
)]
async fn finish_register(
    state: State<AppState>,
    origin: SameOrigin,
    request: Json<SignupFinish>,
) -> Result<FinishReply, AuthError> {
    finish(state, origin, request).await
}

async fn finish(
    State(state): State<AppState>,
    _origin: SameOrigin,
    Json(request): Json<SignupFinish>,
) -> Result<FinishReply, AuthError> {
    let finished = ceremony::finish_registration(
        &state,
        &Secret::from_text(&request.challenge),
        &request.credential,
    )
    .await?;
    let mut tx = state.pool().begin().await?;
    let user_id = create_user(&mut tx, finished).await?;
    let recovery_code = recovery::issue(&mut tx, user_id).await?;
    let token = session::start(&mut tx, user_id).await?;
    tx.commit().await?;
    state.bootstrap().disarm();
    Ok((
        [(SET_COOKIE, session::set_cookie(&token))],
        Json(SignedUp {
            user_id,
            recovery_code: recovery_code.display(),
        }),
    ))
}

async fn create_user(
    conn: &mut PgConnection,
    finished: FinishedRegistration,
) -> Result<UserId, AuthError> {
    let FinishedRegistration {
        purpose,
        passkey_name,
        passkey,
    } = finished;
    let (username, display_name, user_handle, is_admin) = match purpose {
        Purpose::Bootstrap {
            username,
            display_name,
            user_handle,
        } => {
            users::lock_first_user(conn).await?;
            if users::any_exists(conn).await? {
                return Err(AuthError::InvalidToken);
            }
            (username, display_name, user_handle, true)
        }
        Purpose::Invite {
            invite_id,
            username,
            display_name,
            user_handle,
        } => {
            if !invites::consume(conn, invite_id).await? {
                return Err(AuthError::InvalidToken);
            }
            (username, display_name, user_handle, false)
        }
        Purpose::AddPasskey { .. } | Purpose::Recovery { .. } => {
            return Err(AuthError::InvalidToken);
        }
    };
    let user_id = users::insert(
        conn,
        &username,
        &display_name,
        user_handle.as_bytes(),
        is_admin,
    )
    .await
    .map_err(|error| {
        if violated_constraint(&error) == Some("users_username_key") {
            AuthError::NameTaken
        } else {
            AuthError::Database(error)
        }
    })?;
    ceremony::store_passkey(conn, user_id, &passkey, &passkey_name).await?;
    Ok(user_id)
}
