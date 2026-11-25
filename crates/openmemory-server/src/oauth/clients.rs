use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::PgConnection;

use crate::auth::token::Secret;
use crate::oauth::cimd;
use crate::oauth::error::{OAuthError, OAuthErrorCode};
use crate::oauth::fetch::FetchError;
use crate::oauth::ids::OauthClientId;
use crate::oauth::queries::clients::{self, ClientRow};
use crate::oauth::redirect;
use crate::state::AppState;

const DYNAMIC_PREFIX: &str = "dcr-";
const OFFICIAL_CLI: &str = "openmemory-cli";
const MAX_REDIRECT_URIS: usize = 10;
const MAX_NAME_CHARS: usize = 128;
const DEFAULT_NAME: &str = "unnamed client";
const GRANT_TYPES: [&str; 3] = [
    "authorization_code",
    "refresh_token",
    "urn:ietf:params:oauth:grant-type:device_code",
];

#[derive(Debug, Clone)]
pub(crate) struct Client {
    pub(crate) id: OauthClientId,
    pub(crate) public_id: String,
    pub(crate) name: String,
    pub(crate) redirect_uris: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct Registration {
    #[serde(default)]
    redirect_uris: Vec<String>,
    client_name: Option<String>,
    token_endpoint_auth_method: Option<String>,
    grant_types: Option<Vec<String>>,
    response_types: Option<Vec<String>>,
}

impl Client {
    pub(crate) fn is_dynamic(&self) -> bool {
        self.public_id.starts_with(DYNAMIC_PREFIX)
    }

    pub(crate) fn is_official(&self) -> bool {
        self.public_id == OFFICIAL_CLI
    }
}

impl From<ClientRow> for Client {
    fn from(row: ClientRow) -> Self {
        Self {
            id: row.id,
            public_id: row.client_id,
            name: row.name,
            redirect_uris: row.redirect_uris,
        }
    }
}

pub(crate) async fn stored(conn: &mut PgConnection, client_id: &str) -> Result<Client, OAuthError> {
    clients::find(conn, client_id)
        .await?
        .map(Client::from)
        .ok_or(OAuthErrorCode::InvalidClient.into())
}

pub(crate) async fn by_id(
    conn: &mut PgConnection,
    id: OauthClientId,
) -> Result<Client, OAuthError> {
    clients::find_by_id(conn, id)
        .await?
        .map(Client::from)
        .ok_or(OAuthErrorCode::InvalidClient.into())
}

pub(crate) async fn resolve(state: &AppState, client_id: &str) -> Result<Client, OAuthError> {
    let mut conn = state.pool().acquire().await?;
    let Some(url) = cimd::parse_client_id(client_id, state.public_url().origin()) else {
        return stored(&mut conn, client_id).await;
    };
    drop(conn);
    let bytes = state
        .fetcher()
        .fetch_json(&url)
        .await
        .map_err(|error| match error {
            FetchError::Busy => OAuthErrorCode::TemporarilyUnavailable,
            FetchError::Blocked
            | FetchError::Unresolved
            | FetchError::Request
            | FetchError::Status
            | FetchError::TooLarge => OAuthErrorCode::InvalidClient,
        })?;
    let metadata =
        cimd::parse_document(client_id, &bytes).map_err(|_| OAuthErrorCode::InvalidClient)?;
    let mut conn = state.pool().acquire().await?;
    let row = clients::upsert(
        &mut conn,
        client_id,
        &metadata.name,
        &metadata.redirect_uris,
    )
    .await?;
    Ok(row.into())
}

pub(super) async fn register(
    State(state): State<AppState>,
    Json(request): Json<Registration>,
) -> Result<(StatusCode, Json<Value>), OAuthError> {
    validate(&request)?;
    let name = request
        .client_name
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or(DEFAULT_NAME);
    let client_id = format!("{DYNAMIC_PREFIX}{}", Secret::generate().expose());
    let mut conn = state.pool().acquire().await?;
    let row = clients::upsert(&mut conn, &client_id, name, &request.redirect_uris).await?;
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "client_id": row.client_id,
            "client_name": row.name,
            "redirect_uris": row.redirect_uris,
            "token_endpoint_auth_method": "none",
            "grant_types": request.grant_types.unwrap_or_else(|| vec!["authorization_code".to_owned()]),
            "response_types": ["code"],
        })),
    ))
}

fn validate(request: &Registration) -> Result<(), OAuthErrorCode> {
    let uris = &request.redirect_uris;
    if uris.is_empty()
        || uris.len() > MAX_REDIRECT_URIS
        || uris
            .iter()
            .any(|uri| redirect::parse_registered(uri).is_none())
    {
        return Err(OAuthErrorCode::InvalidRedirectUri);
    }
    let name_ok = request.client_name.as_deref().is_none_or(|name| {
        name.chars().count() <= MAX_NAME_CHARS && !name.chars().any(char::is_control)
    });
    let method_ok = request
        .token_endpoint_auth_method
        .as_deref()
        .is_none_or(|method| method == "none");
    let grants_ok = request.grant_types.as_ref().is_none_or(|grants| {
        !grants.is_empty() && grants.iter().all(|g| GRANT_TYPES.contains(&g.as_str()))
    });
    let responses_ok = request
        .response_types
        .as_ref()
        .is_none_or(|types| types.iter().all(|t| t == "code"));
    if name_ok && method_ok && grants_ok && responses_ok {
        Ok(())
    } else {
        Err(OAuthErrorCode::InvalidClientMetadata)
    }
}
