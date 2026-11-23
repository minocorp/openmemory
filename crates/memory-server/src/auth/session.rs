use axum::extract::FromRequestParts;
use axum::http::HeaderValue;
use axum::http::header::COOKIE;
use axum::http::request::Parts;
use sqlx::PgConnection;

use crate::access::{Actor, Scope, UserId};
use crate::auth::ids::SessionId;
use crate::auth::queries::sessions;
use crate::auth::token::Secret;
use crate::auth::{AuthError, csrf};
use crate::state::AppState;

const COOKIE_NAME: &str = "__Host-session";
const IDLE_SECONDS: f64 = 7.0 * 24.0 * 3600.0;
const ABSOLUTE_LIFETIME: u32 = 30 * 24 * 3600;
const TOUCH_SECONDS: f64 = 300.0;

#[derive(Debug, Clone)]
pub(crate) struct WebSession {
    pub(crate) actor: Actor,
    pub(crate) display_name: String,
    pub(crate) is_admin: bool,
    session_id: SessionId,
}

impl WebSession {
    pub(crate) fn user_id(&self) -> UserId {
        self.actor.user_id
    }

    pub(super) fn id(&self) -> SessionId {
        self.session_id
    }

    pub(crate) fn require_admin(&self) -> Result<(), AuthError> {
        if self.is_admin {
            Ok(())
        } else {
            Err(AuthError::Forbidden)
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct BinarySession(pub(crate) WebSession);

impl FromRequestParts<AppState> for WebSession {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        csrf::check(parts, state.public_url().issuer())?;
        authenticate(parts, state).await
    }
}

impl FromRequestParts<AppState> for BinarySession {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        csrf::check_origin(parts, state.public_url().issuer())?;
        authenticate(parts, state).await.map(Self)
    }
}

async fn authenticate(parts: &Parts, state: &AppState) -> Result<WebSession, AuthError> {
    let token = cookie_token(parts).ok_or(AuthError::Unauthenticated)?;
    let mut conn = state.pool().acquire().await?;
    let row = sessions::find_live(&mut conn, &token.hash(), IDLE_SECONDS, TOUCH_SECONDS)
        .await?
        .ok_or(AuthError::Unauthenticated)?;
    if row.is_stale {
        sessions::touch(&mut conn, row.id).await?;
    }
    Ok(WebSession {
        actor: Actor {
            user_id: row.user_id,
            connection_id: None,
            name: row.username,
            scope: Scope::everything(),
        },
        display_name: row.display_name,
        is_admin: row.is_admin,
        session_id: row.id,
    })
}

pub(crate) async fn start(conn: &mut PgConnection, user_id: UserId) -> Result<Secret, AuthError> {
    sessions::delete_expired(conn).await?;
    let token = Secret::generate();
    sessions::insert(conn, user_id, &token.hash(), f64::from(ABSOLUTE_LIFETIME)).await?;
    Ok(token)
}

pub(crate) async fn end(conn: &mut PgConnection, session: &WebSession) -> Result<(), AuthError> {
    sessions::delete(conn, session.id()).await?;
    Ok(())
}

pub(crate) async fn end_all(conn: &mut PgConnection, user_id: UserId) -> Result<(), AuthError> {
    sessions::delete_for_user(conn, user_id).await?;
    Ok(())
}

pub(crate) fn set_cookie(token: &Secret) -> HeaderValue {
    cookie_header(&format!(
        "{COOKIE_NAME}={}; Max-Age={}; Path=/; Secure; HttpOnly; SameSite=Lax",
        token.expose(),
        ABSOLUTE_LIFETIME,
    ))
}

pub(crate) fn clear_cookie() -> HeaderValue {
    cookie_header(&format!(
        "{COOKIE_NAME}=; Max-Age=0; Path=/; Secure; HttpOnly; SameSite=Lax"
    ))
}

fn cookie_header(value: &str) -> HeaderValue {
    HeaderValue::from_str(value).unwrap_or_else(|_| HeaderValue::from_static(""))
}

fn cookie_token(parts: &Parts) -> Option<Secret> {
    parts
        .headers
        .get_all(COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|header| header.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == COOKIE_NAME)
        .map(|(_, value)| Secret::from_text(value))
}
