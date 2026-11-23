use axum::Json;
use axum::extract::State;
use serde_json::{Value, json};

use crate::state::AppState;

pub(super) const PROTECTED_RESOURCE_PATH: &str = "/.well-known/oauth-protected-resource/mcp";

pub(crate) fn resource_metadata_url(state: &AppState) -> String {
    state.public_url().join(PROTECTED_RESOURCE_PATH)
}

pub(super) async fn authorization_server(State(state): State<AppState>) -> Json<Value> {
    let url = state.public_url();
    Json(json!({
        "issuer": url.issuer(),
        "authorization_endpoint": url.join("/oauth/authorize"),
        "token_endpoint": url.join("/oauth/token"),
        "registration_endpoint": url.join("/oauth/register"),
        "revocation_endpoint": url.join("/oauth/revoke"),
        "device_authorization_endpoint": url.join("/oauth/device"),
        "response_types_supported": ["code"],
        "grant_types_supported": [
            "authorization_code",
            "refresh_token",
            "urn:ietf:params:oauth:grant-type:device_code",
        ],
        "code_challenge_methods_supported": ["S256"],
        "token_endpoint_auth_methods_supported": ["none"],
        "revocation_endpoint_auth_methods_supported": ["none"],
        "client_id_metadata_document_supported": true,
        "authorization_response_iss_parameter_supported": true,
    }))
}

pub(super) async fn protected_resource(State(state): State<AppState>) -> Json<Value> {
    let url = state.public_url();
    Json(json!({
        "resource": url.resource(),
        "authorization_servers": [url.issuer()],
        "bearer_methods_supported": ["header"],
    }))
}
