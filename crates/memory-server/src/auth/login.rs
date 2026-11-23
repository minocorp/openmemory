use axum::extract::State;
use axum::http::header::SET_COOKIE;
use axum::http::{HeaderName, HeaderValue};
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};
use webauthn_rs::prelude::{
    CreationChallengeResponse, PublicKeyCredential, RegisterPublicKeyCredential,
    RequestChallengeResponse,
};

use crate::access::{UserId, Username, connections};
use crate::auth::ceremony::{self, Assertion, Purpose};
use crate::auth::queries::{passkeys as passkey_queries, users};
use crate::auth::token::Secret;
use crate::auth::{AuthError, SameOrigin, recovery, session};
use crate::error::ErrorBody;
use crate::state::AppState;

const RECOVERED_PASSKEY_NAME: &str = "recovered passkey";

type SessionCookie = [(HeaderName, HeaderValue); 1];

#[derive(Debug, Deserialize, ToSchema)]
struct LoginStart {
    username: Username,
}

#[derive(Debug, Serialize, ToSchema)]
struct LoginStarted {
    challenge: String,
    #[schema(value_type = Object)]
    options: RequestChallengeResponse,
}

#[derive(Debug, Deserialize, ToSchema)]
struct LoginFinish {
    challenge: String,
    #[schema(value_type = Object)]
    credential: PublicKeyCredential,
}

#[derive(Debug, Serialize, ToSchema)]
struct LoggedIn {
    user_id: UserId,
}

#[derive(Debug, Deserialize, ToSchema)]
struct RecoveryStart {
    username: Username,
    code: String,
}

#[derive(Debug, Serialize, ToSchema)]
struct RecoveryStarted {
    challenge: String,
    #[schema(value_type = Object)]
    options: CreationChallengeResponse,
}

#[derive(Debug, Deserialize, ToSchema)]
struct RecoveryFinish {
    challenge: String,
    #[schema(value_type = Object)]
    credential: RegisterPublicKeyCredential,
}

#[derive(Debug, Serialize, ToSchema)]
struct Recovered {
    user_id: UserId,
    recovery_code: String,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        start_login,
        finish_login,
        start_discoverable_login,
        finish_discoverable_login,
        start_recovery,
        finish_recovery
    ),
    components(schemas(
        LoginStart,
        LoginStarted,
        LoginFinish,
        LoggedIn,
        RecoveryStart,
        RecoveryStarted,
        RecoveryFinish,
        Recovered
    ))
)]
pub(super) struct Doc;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/auth/login/start", post(start_login))
        .route("/api/auth/login/finish", post(finish_login))
        .route(
            "/api/auth/login/discoverable/start",
            post(start_discoverable_login),
        )
        .route(
            "/api/auth/login/discoverable/finish",
            post(finish_discoverable_login),
        )
        .route("/api/auth/recovery/start", post(start_recovery))
        .route("/api/auth/recovery/finish", post(finish_recovery))
}

#[utoipa::path(
    post,
    path = "/api/auth/login/start",
    tag = "auth",
    request_body = LoginStart,
    responses((status = 200, body = LoginStarted), (status = 403, body = ErrorBody))
)]
async fn start_login(
    State(state): State<AppState>,
    _origin: SameOrigin,
    Json(request): Json<LoginStart>,
) -> Result<Json<LoginStarted>, AuthError> {
    let mut conn = state.pool().acquire().await?;
    let user = users::find_by_username(&mut conn, request.username.as_str()).await?;
    drop(conn);
    let started = match user {
        Some(user) => ceremony::start_authentication(&state, user.id, Assertion::Login).await,
        None => Err(AuthError::InvalidCredentials),
    };
    // Unknown names and users without passkeys get a decoy so the response does not reveal which names exist.
    let (challenge, options) = match started {
        Err(AuthError::InvalidCredentials) => {
            ceremony::start_decoy_authentication(&state, request.username.as_str()).await?
        }
        other => other?,
    };
    Ok(Json(LoginStarted {
        challenge: challenge.expose().to_owned(),
        options,
    }))
}

#[utoipa::path(
    post,
    path = "/api/auth/login/finish",
    tag = "auth",
    request_body = LoginFinish,
    responses(
        (status = 200, body = LoggedIn, headers(("Set-Cookie" = String))),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
    )
)]
async fn finish_login(
    State(state): State<AppState>,
    _origin: SameOrigin,
    Json(request): Json<LoginFinish>,
) -> Result<(SessionCookie, Json<LoggedIn>), AuthError> {
    let user_id = ceremony::finish_authentication(
        &state,
        &Secret::from_text(&request.challenge),
        &request.credential,
        Assertion::Login,
    )
    .await?;
    let mut conn = state.pool().acquire().await?;
    let token = session::start(&mut conn, user_id).await?;
    Ok((
        [(SET_COOKIE, session::set_cookie(&token))],
        Json(LoggedIn { user_id }),
    ))
}

#[utoipa::path(
    post,
    path = "/api/auth/login/discoverable/start",
    tag = "auth",
    responses((status = 200, body = LoginStarted), (status = 403, body = ErrorBody))
)]
async fn start_discoverable_login(
    State(state): State<AppState>,
    _origin: SameOrigin,
) -> Result<Json<LoginStarted>, AuthError> {
    let (challenge, options) = ceremony::start_discoverable_authentication(&state).await?;
    Ok(Json(LoginStarted {
        challenge: challenge.expose().to_owned(),
        options,
    }))
}

#[utoipa::path(
    post,
    path = "/api/auth/login/discoverable/finish",
    tag = "auth",
    request_body = LoginFinish,
    responses(
        (status = 200, body = LoggedIn, headers(("Set-Cookie" = String))),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
    )
)]
async fn finish_discoverable_login(
    State(state): State<AppState>,
    _origin: SameOrigin,
    Json(request): Json<LoginFinish>,
) -> Result<(SessionCookie, Json<LoggedIn>), AuthError> {
    let user_id = ceremony::finish_discoverable_authentication(
        &state,
        &Secret::from_text(&request.challenge),
        &request.credential,
    )
    .await?;
    let mut conn = state.pool().acquire().await?;
    let token = session::start(&mut conn, user_id).await?;
    Ok((
        [(SET_COOKIE, session::set_cookie(&token))],
        Json(LoggedIn { user_id }),
    ))
}

#[utoipa::path(
    post,
    path = "/api/auth/recovery/start",
    tag = "auth",
    request_body = RecoveryStart,
    responses((status = 200, body = RecoveryStarted), (status = 401, body = ErrorBody))
)]
async fn start_recovery(
    State(state): State<AppState>,
    _origin: SameOrigin,
    Json(request): Json<RecoveryStart>,
) -> Result<Json<RecoveryStarted>, AuthError> {
    let mut conn = state.pool().acquire().await?;
    let user = users::find_by_username(&mut conn, request.username.as_str())
        .await?
        .ok_or(AuthError::InvalidCredentials)?;
    let code = recovery::RecoveryCode::parse(&request.code);
    if !recovery::is_valid(&mut conn, user.id, &code).await? {
        return Err(AuthError::InvalidCredentials);
    }
    let existing = ceremony::load_passkeys(&mut conn, user.id).await?;
    let purpose = Purpose::Recovery {
        user_id: user.id,
        code_hash: code.hash().as_bytes().to_vec(),
    };
    let (challenge, options) = ceremony::start_registration(
        &state,
        ceremony::Account {
            user_handle: ceremony::user_handle(&user.user_handle)?,
            username: &user.username,
            display_name: &user.display_name,
        },
        existing
            .iter()
            .map(|passkey| passkey.cred_id().clone())
            .collect(),
        purpose,
        RECOVERED_PASSKEY_NAME.to_owned(),
    )
    .await?;
    Ok(Json(RecoveryStarted {
        challenge: challenge.expose().to_owned(),
        options,
    }))
}

#[utoipa::path(
    post,
    path = "/api/auth/recovery/finish",
    tag = "auth",
    request_body = RecoveryFinish,
    responses(
        (status = 200, body = Recovered, headers(("Set-Cookie" = String))),
        (status = 400, body = ErrorBody),
    )
)]
async fn finish_recovery(
    State(state): State<AppState>,
    _origin: SameOrigin,
    Json(request): Json<RecoveryFinish>,
) -> Result<(SessionCookie, Json<Recovered>), AuthError> {
    let finished = ceremony::finish_registration(
        &state,
        &Secret::from_text(&request.challenge),
        &request.credential,
    )
    .await?;
    let Purpose::Recovery { user_id, code_hash } = finished.purpose else {
        return Err(AuthError::InvalidToken);
    };
    let code_hash =
        crate::auth::token::SecretHash::from_slice(&code_hash).ok_or(AuthError::InvalidToken)?;
    let mut tx = state.pool().begin().await?;
    if !recovery::consume(&mut tx, user_id, &code_hash).await? {
        return Err(AuthError::InvalidToken);
    }
    ceremony::store_passkey(&mut tx, user_id, &finished.passkey, &finished.passkey_name).await?;
    passkey_queries::delete_all_except(&mut tx, user_id, finished.passkey.cred_id().as_ref())
        .await?;
    session::end_all(&mut tx, user_id).await?;
    connections::revoke_all_for_user(&mut tx, user_id).await?;
    let new_code = recovery::issue(&mut tx, user_id).await?;
    let token = session::start(&mut tx, user_id).await?;
    tx.commit().await?;
    Ok((
        [(SET_COOKIE, session::set_cookie(&token))],
        Json(Recovered {
            user_id,
            recovery_code: new_code.display(),
        }),
    ))
}
