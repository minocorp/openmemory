mod changes;
mod error;
mod files;
mod pages;
mod precondition;
mod purge;
mod range;
mod search;
mod system;
mod tree;
mod upload_links;

#[cfg(test)]
mod tests;

use axum::routing::get;
use axum::{Json, Router};
use utoipa::openapi::security::{ApiKey, ApiKeyValue, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::error::ErrorBody;
use crate::state::AppState;

const SESSION_SCHEME: &str = "session";
const SESSION_COOKIE: &str = "__Host-session";

#[derive(OpenApi)]
#[openapi(
    paths(openapi_document),
    components(schemas(ErrorBody)),
    modifiers(&DocumentSetup)
)]
struct Root;

struct DocumentSetup;

impl Modify for DocumentSetup {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        "Memory API".clone_into(&mut openapi.info.title);
        openapi.info.description = None;
        openapi.info.license = None;
        openapi
            .components
            .get_or_insert_with(Default::default)
            .add_security_scheme(
                SESSION_SCHEME,
                SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new(SESSION_COOKIE))),
            );
    }
}

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/openapi.json", get(openapi_document))
        .merge(pages::router())
        .merge(files::router())
        .merge(upload_links::router())
        .merge(tree::router())
        .merge(search::router())
        .merge(changes::router())
        .merge(system::router())
        .merge(purge::router())
}

pub(crate) fn openapi() -> utoipa::openapi::OpenApi {
    let mut document = Root::openapi();
    for part in [
        pages::Doc::openapi(),
        files::Doc::openapi(),
        upload_links::Doc::openapi(),
        tree::Doc::openapi(),
        search::Doc::openapi(),
        changes::Doc::openapi(),
        system::Doc::openapi(),
        purge::Doc::openapi(),
        crate::auth::openapi(),
        crate::oauth::openapi(),
        <crate::access::connections::Doc as OpenApi>::openapi(),
    ] {
        document.merge(part);
    }
    document
}

#[utoipa::path(
    get,
    path = "/api/openapi.json",
    tag = "meta",
    responses((status = 200, description = "This document", content_type = "application/json"))
)]
async fn openapi_document() -> Json<utoipa::openapi::OpenApi> {
    Json(openapi())
}
