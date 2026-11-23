mod personal;
mod queries;
pub(crate) mod scopes;

use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use time::OffsetDateTime;
use utoipa::{OpenApi, ToSchema};

use crate::access::{Actor, Audience, ConnectionId, Folders, Name, UserId};
use crate::auth::token::SecretHash;
use crate::auth::{AuthError, WebSession, violated_constraint};
use crate::error::ErrorBody;
use crate::state::AppState;
use queries::ConnectionRow;

const NAME_KEY: &str = "connections_user_id_name_live_key";

#[derive(Debug, thiserror::Error)]
pub(crate) enum ConnectionError {
    #[error("connection not found")]
    NotFound,
    #[error("connection name is taken")]
    NameTaken,
    #[error("scopes are invalid")]
    InvalidScopes(#[from] scopes::ScopesError),
    #[error("database failure")]
    Database(#[from] sqlx::Error),
}

impl From<ConnectionError> for AuthError {
    fn from(error: ConnectionError) -> Self {
        match error {
            ConnectionError::NotFound => Self::NotFound,
            ConnectionError::NameTaken => Self::NameTaken,
            ConnectionError::InvalidScopes(_) => Self::InvalidInput("scopes"),
            ConnectionError::Database(error) => Self::Database(error),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
struct ConnectionView {
    id: ConnectionId,
    user_id: UserId,
    owner: String,
    owner_display_name: String,
    name: String,
    scopes: Vec<String>,
    can_write: bool,
    #[serde(with = "time::serde::rfc3339")]
    created_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    last_used_at: Option<OffsetDateTime>,
    kind: ConnectionKind,
    audience: Audience,
    #[serde(with = "time::serde::rfc3339::option")]
    expires_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
enum ConnectionKind {
    Oauth,
    Token,
}

#[derive(Debug, Deserialize, ToSchema)]
struct ConnectionUpdate {
    name: Option<Name>,
    scopes: Option<Vec<String>>,
    can_write: Option<bool>,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        list_connections,
        update_connection,
        revoke_connection,
        personal::create_token
    ),
    components(schemas(
        ConnectionView,
        ConnectionKind,
        ConnectionUpdate,
        Audience,
        personal::TokenCreate,
        personal::TokenCreated
    ))
)]
pub(crate) struct Doc;

impl From<ConnectionRow> for ConnectionView {
    fn from(row: ConnectionRow) -> Self {
        Self {
            id: row.id,
            user_id: row.user_id,
            owner: row.owner,
            owner_display_name: row.owner_display_name,
            name: row.name,
            scopes: row.scopes,
            can_write: row.can_write,
            created_at: row.created_at,
            last_used_at: row.last_used_at,
            kind: if row.is_token {
                ConnectionKind::Token
            } else {
                ConnectionKind::Oauth
            },
            audience: row.audience,
            expires_at: row.expires_at,
        }
    }
}

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/connections", get(list_connections))
        .route("/api/tokens", axum::routing::post(personal::create_token))
        .route(
            "/api/connections/{id}",
            axum::routing::patch(update_connection).delete(revoke_connection),
        )
}

pub(crate) async fn create(
    conn: &mut PgConnection,
    user_id: UserId,
    name: &Name,
    folders: &Folders,
    can_write: bool,
    audience: Audience,
) -> Result<ConnectionId, ConnectionError> {
    queries::insert(
        conn,
        user_id,
        name.as_str(),
        &scopes::encode(folders),
        can_write,
        audience,
    )
    .await
    .map_err(name_conflict)
}

pub(crate) async fn create_personal(
    conn: &mut PgConnection,
    user_id: UserId,
    name: &Name,
    folders: &Folders,
    can_write: bool,
    token: &SecretHash,
    lifetime_seconds: f64,
) -> Result<(ConnectionId, OffsetDateTime), ConnectionError> {
    queries::insert_personal(
        conn,
        user_id,
        name.as_str(),
        &scopes::encode(folders),
        can_write,
        token,
        lifetime_seconds,
    )
    .await
    .map_err(name_conflict)
}

pub(crate) async fn find_by_personal_token(
    conn: &mut PgConnection,
    token: &SecretHash,
) -> Result<Option<ConnectionId>, sqlx::Error> {
    queries::find_by_personal_token(conn, token).await
}

pub(crate) async fn proposed_name(
    conn: &mut PgConnection,
    user_id: UserId,
    base: &str,
) -> Result<Name, sqlx::Error> {
    let slug = slug(base);
    for suffix in 1_u32.. {
        let candidate = if suffix == 1 {
            slug.clone()
        } else {
            format!("{slug}-{suffix}")
        };
        if !queries::name_taken(conn, user_id, &candidate).await? {
            if let Ok(name) = Name::parse(&candidate) {
                return Ok(name);
            }
            break;
        }
    }
    Ok(Name::parse("agent").unwrap_or_else(|_| unreachable!()))
}

pub(crate) async fn delete_unexchanged(
    conn: &mut PgConnection,
    older_than_seconds: f64,
) -> Result<u64, sqlx::Error> {
    queries::delete_unexchanged(conn, older_than_seconds).await
}

pub(crate) async fn revoke_all_for_user(
    conn: &mut PgConnection,
    user_id: UserId,
) -> Result<(), sqlx::Error> {
    queries::revoke_all_for_user(conn, user_id).await
}

pub(crate) async fn touch(conn: &mut PgConnection, id: ConnectionId) -> Result<(), sqlx::Error> {
    queries::touch(conn, id).await
}

pub(crate) async fn revoke_one(
    conn: &mut PgConnection,
    id: ConnectionId,
) -> Result<(), sqlx::Error> {
    queries::revoke(conn, id).await
}

pub(crate) async fn load_actor_in(
    conn: &mut PgConnection,
    id: ConnectionId,
    audience: Option<Audience>,
) -> Result<Option<Actor>, ConnectionError> {
    let Some(row) = queries::find_live(conn, id)
        .await?
        .filter(|row| audience.is_none_or(|wanted| row.audience == wanted))
    else {
        return Ok(None);
    };
    Ok(Some(Actor {
        user_id: row.user_id,
        connection_id: Some(row.id),
        name: format!("{}/{}", row.owner, row.name),
        scope: scopes::to_scope(&row.scopes, row.can_write)?,
    }))
}

fn slug(text: &str) -> String {
    let mut slug = String::new();
    for character in text.chars().flat_map(char::to_lowercase) {
        if character.is_alphanumeric() {
            slug.push(character);
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    let trimmed: String = slug.trim_end_matches('-').chars().take(48).collect();
    if trimmed.is_empty() {
        "agent".to_owned()
    } else {
        trimmed
    }
}

fn name_conflict(error: sqlx::Error) -> ConnectionError {
    if violated_constraint(&error) == Some(NAME_KEY) {
        ConnectionError::NameTaken
    } else {
        ConnectionError::Database(error)
    }
}

async fn manageable(
    conn: &mut PgConnection,
    session: &WebSession,
    id: ConnectionId,
) -> Result<ConnectionRow, ConnectionError> {
    let row = queries::find_live(conn, id)
        .await?
        .ok_or(ConnectionError::NotFound)?;
    if row.user_id == session.actor.user_id || session.is_admin {
        Ok(row)
    } else {
        Err(ConnectionError::NotFound)
    }
}

#[utoipa::path(
    get,
    path = "/api/connections",
    tag = "connections",
    security(("session" = [])),
    responses((status = 200, body = Vec<ConnectionView>), (status = 401, body = ErrorBody))
)]
async fn list_connections(
    State(state): State<AppState>,
    session: WebSession,
) -> Result<Json<Vec<ConnectionView>>, AuthError> {
    let mut conn = state.pool().acquire().await?;
    let only = (!session.is_admin).then_some(session.actor.user_id);
    let rows = queries::list_live(&mut conn, only).await?;
    Ok(Json(rows.into_iter().map(ConnectionView::from).collect()))
}

#[utoipa::path(
    patch,
    path = "/api/connections/{id}",
    tag = "connections",
    security(("session" = [])),
    params(("id" = i64, Path)),
    request_body = ConnectionUpdate,
    responses(
        (status = 200, body = ConnectionView),
        (status = 400, body = ErrorBody),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 409, body = ErrorBody),
    )
)]
async fn update_connection(
    State(state): State<AppState>,
    session: WebSession,
    Path(id): Path<ConnectionId>,
    Json(update): Json<ConnectionUpdate>,
) -> Result<Json<ConnectionView>, AuthError> {
    let mut tx = state.pool().begin().await?;
    manageable(&mut tx, &session, id).await?;
    let encoded = match &update.scopes {
        Some(raw) => {
            scopes::decode(raw).map_err(ConnectionError::from)?;
            Some(raw.as_slice())
        }
        None => None,
    };
    queries::update(
        &mut tx,
        id,
        update.name.as_ref().map(Name::as_str),
        encoded,
        update.can_write,
    )
    .await
    .map_err(name_conflict)?;
    let row = manageable(&mut tx, &session, id).await?;
    tx.commit().await?;
    Ok(Json(row.into()))
}

#[utoipa::path(
    delete,
    path = "/api/connections/{id}",
    tag = "connections",
    security(("session" = [])),
    params(("id" = i64, Path)),
    responses((status = 204), (status = 401, body = ErrorBody), (status = 404, body = ErrorBody))
)]
async fn revoke_connection(
    State(state): State<AppState>,
    session: WebSession,
    Path(id): Path<ConnectionId>,
) -> Result<StatusCode, AuthError> {
    let mut tx = state.pool().begin().await?;
    manageable(&mut tx, &session, id).await?;
    queries::revoke(&mut tx, id).await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
