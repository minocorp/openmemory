use axum::extract::State;
use axum::http::HeaderMap;
use axum::http::header::{AUTHORIZATION, CACHE_CONTROL, PRAGMA};
use axum::response::{IntoResponse, Response};
use axum::{Form, Json};
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;

use crate::access::{Audience, ConnectionId, connections};
use crate::auth::token::Secret;
use crate::oauth::clients::{self, Client};
use crate::oauth::error::{OAuthError, OAuthErrorCode};
use crate::oauth::ids::OauthClientId;
use crate::oauth::pkce::CodeChallenge;
use crate::oauth::queries::{codes, devices, tokens};
use crate::state::AppState;

const ACCESS_SECONDS: u32 = 3600;
const REFRESH_SECONDS: u32 = 30 * 24 * 3600;
const DEVICE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";
const POLL_TOLERANCE_SECONDS: f64 = 4.0;

#[derive(Debug, Deserialize)]
pub(super) struct TokenRequest {
    grant_type: Option<String>,
    client_id: Option<String>,
    code: Option<String>,
    redirect_uri: Option<String>,
    code_verifier: Option<String>,
    refresh_token: Option<String>,
    device_code: Option<String>,
    resource: Option<String>,
}

#[derive(Debug, Serialize)]
struct Issued {
    access_token: String,
    token_type: &'static str,
    expires_in: u32,
    refresh_token: String,
}

pub(super) async fn token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Form(request): Form<TokenRequest>,
) -> Result<Response, OAuthError> {
    if headers.contains_key(AUTHORIZATION) {
        return Err(OAuthErrorCode::InvalidClient.into());
    }
    let requested = match request.resource.as_deref() {
        Some(resource) => Some(
            state
                .public_url()
                .audience_of(resource)
                .ok_or(OAuthErrorCode::InvalidTarget)?,
        ),
        None => None,
    };
    let client_id = request
        .client_id
        .as_deref()
        .ok_or(OAuthErrorCode::InvalidRequest)?;
    let mut conn = state.pool().acquire().await?;
    let client = clients::stored(&mut conn, client_id).await?;
    drop(conn);
    let issued = match request.grant_type.as_deref() {
        Some("authorization_code") => {
            authorization_code(&state, &client, &request, requested).await?
        }
        Some("refresh_token") => refresh(&state, &client, &request, requested).await?,
        Some(DEVICE_GRANT) => device_code(&state, &client, &request, requested).await?,
        Some(_) | None => return Err(OAuthErrorCode::UnsupportedGrantType.into()),
    };
    Ok((
        [(CACHE_CONTROL, "no-store"), (PRAGMA, "no-cache")],
        Json(issued),
    )
        .into_response())
}

async fn authorization_code(
    state: &AppState,
    client: &Client,
    request: &TokenRequest,
    requested: Option<Audience>,
) -> Result<Issued, OAuthError> {
    let (Some(code), Some(redirect_uri), Some(verifier)) = (
        request.code.as_deref(),
        request.redirect_uri.as_deref(),
        request.code_verifier.as_deref(),
    ) else {
        return Err(OAuthErrorCode::InvalidRequest.into());
    };
    let hash = Secret::from_text(code).hash();
    let mut conn = state.pool().acquire().await?;
    let Some(row) = codes::burn(&mut conn, &hash).await? else {
        // OAuth 2.1 section 4.1.3: a replayed code means the issued tokens are suspect.
        if let Some(connection_id) = codes::connection_of_used(&mut conn, &hash).await? {
            connections::revoke_one(&mut conn, connection_id).await?;
        }
        return Err(OAuthErrorCode::InvalidGrant.into());
    };
    let challenge = CodeChallenge::from_stored(row.code_challenge);
    if row.is_expired
        || row.oauth_client_id != client.id
        || row.redirect_uri != redirect_uri
        || !challenge.verifies(verifier)
        || !is_live(&mut conn, row.connection_id, requested).await?
    {
        return Err(OAuthErrorCode::InvalidGrant.into());
    }
    issue(&mut conn, row.connection_id, client.id).await
}

async fn refresh(
    state: &AppState,
    client: &Client,
    request: &TokenRequest,
    requested: Option<Audience>,
) -> Result<Issued, OAuthError> {
    let refresh_token = request
        .refresh_token
        .as_deref()
        .ok_or(OAuthErrorCode::InvalidRequest)?;
    let mut tx = state.pool().begin().await?;
    let row = tokens::lock_refresh(&mut tx, &Secret::from_text(refresh_token).hash())
        .await?
        .ok_or(OAuthErrorCode::InvalidGrant)?;
    if row.oauth_client_id != client.id {
        return Err(OAuthErrorCode::InvalidGrant.into());
    }
    if row.is_rotated {
        // OAuth 2.1 section 4.3.1: a reused refresh token revokes the whole grant.
        connections::revoke_one(&mut tx, row.connection_id).await?;
        tx.commit().await?;
        return Err(OAuthErrorCode::InvalidGrant.into());
    }
    if row.is_expired || !is_live(&mut tx, row.connection_id, requested).await? {
        return Err(OAuthErrorCode::InvalidGrant.into());
    }
    tokens::mark_rotated(&mut tx, row.id).await?;
    let issued = issue(&mut tx, row.connection_id, client.id).await?;
    tx.commit().await?;
    Ok(issued)
}

async fn device_code(
    state: &AppState,
    client: &Client,
    request: &TokenRequest,
    requested: Option<Audience>,
) -> Result<Issued, OAuthError> {
    let device_code = request
        .device_code
        .as_deref()
        .ok_or(OAuthErrorCode::InvalidRequest)?;
    let mut tx = state.pool().begin().await?;
    let row = devices::lock_by_device_code(
        &mut tx,
        &Secret::from_text(device_code).hash(),
        POLL_TOLERANCE_SECONDS,
    )
    .await?
    .ok_or(OAuthErrorCode::InvalidGrant)?;
    if row.oauth_client_id != client.id {
        return Err(OAuthErrorCode::InvalidGrant.into());
    }
    let outcome = if row.is_expired {
        Some(OAuthErrorCode::ExpiredToken)
    } else if row.is_denied {
        Some(OAuthErrorCode::AccessDenied)
    } else {
        None
    };
    if let Some(code) = outcome {
        devices::delete(&mut tx, row.id).await?;
        tx.commit().await?;
        return Err(code.into());
    }
    let pending = if row.is_polling_too_fast {
        Some(OAuthErrorCode::SlowDown)
    } else if row.connection_id.is_none() {
        Some(OAuthErrorCode::AuthorizationPending)
    } else {
        None
    };
    if let Some(code) = pending {
        devices::record_poll(&mut tx, row.id).await?;
        tx.commit().await?;
        return Err(code.into());
    }
    let connection_id = row.connection_id.ok_or(OAuthErrorCode::InvalidGrant)?;
    devices::delete(&mut tx, row.id).await?;
    if !is_live(&mut tx, connection_id, requested).await? {
        tx.commit().await?;
        return Err(OAuthErrorCode::InvalidGrant.into());
    }
    let issued = issue(&mut tx, connection_id, client.id).await?;
    tx.commit().await?;
    Ok(issued)
}

async fn is_live(
    conn: &mut PgConnection,
    id: ConnectionId,
    requested: Option<Audience>,
) -> Result<bool, OAuthError> {
    Ok(connections::load_actor_in(conn, id, requested)
        .await?
        .is_some())
}

async fn issue(
    conn: &mut PgConnection,
    connection_id: ConnectionId,
    client: OauthClientId,
) -> Result<Issued, OAuthError> {
    let access = Secret::generate();
    let refresh = Secret::generate();
    tokens::insert(
        conn,
        connection_id,
        client,
        &access.hash(),
        &refresh.hash(),
        f64::from(ACCESS_SECONDS),
        f64::from(REFRESH_SECONDS),
    )
    .await?;
    Ok(Issued {
        access_token: access.expose().to_owned(),
        token_type: "Bearer",
        expires_in: ACCESS_SECONDS,
        refresh_token: refresh.expose().to_owned(),
    })
}
