mod authorize;
mod bearer;
mod cimd;
mod clients;
mod consent;
mod device;
mod error;
mod expiry;
mod fetch;
mod ids;
mod metadata;
mod pkce;
mod queries;
mod redirect;
mod revoke;
mod token;

use axum::Router;
use axum::routing::{get, post};

use crate::state::AppState;

pub(crate) use bearer::McpActor;
pub(crate) use expiry::expire_pending;
pub(crate) use fetch::ClientFetcher;

pub(crate) fn openapi() -> utoipa::openapi::OpenApi {
    <consent::Doc as utoipa::OpenApi>::openapi()
}

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/.well-known/oauth-authorization-server",
            get(metadata::authorization_server),
        )
        .route(
            metadata::PROTECTED_RESOURCE_PATH,
            get(metadata::protected_resource),
        )
        .route("/oauth/register", post(clients::register))
        .route("/oauth/authorize", get(authorize::authorize))
        .route("/oauth/token", post(token::token))
        .route("/oauth/revoke", post(revoke::revoke))
        .route("/oauth/device", post(device::start))
        .merge(consent::router())
}
