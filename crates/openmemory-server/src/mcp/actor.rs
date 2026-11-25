use axum::http::request::Parts;
use rmcp::ErrorData;

use crate::access::Actor;
use crate::oauth::McpActor;

pub(super) fn of(parts: &Parts) -> Result<&Actor, ErrorData> {
    parts
        .extensions
        .get::<McpActor>()
        .map(|actor| &actor.0)
        .ok_or_else(|| ErrorData::internal_error("request has no authenticated actor", None))
}
