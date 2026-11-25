use std::sync::Arc;

use axum::body::Body;
use axum::extract::Request;
use axum::response::Response;
use axum::routing::any;
use axum::{Extension, Router};
use rmcp::transport::streamable_http_server::session::never::NeverSessionManager;
use rmcp::transport::streamable_http_server::{StreamableHttpServerConfig, StreamableHttpService};

use crate::config::PublicUrl;
use crate::mcp::server::OpenMemoryServer;
use crate::oauth::McpActor;
use crate::state::AppState;

const LOOPBACK_HOSTS: [&str; 3] = ["localhost", "127.0.0.1", "::1"];

type OpenMemoryService = StreamableHttpService<OpenMemoryServer, NeverSessionManager>;

pub(crate) fn router(state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/mcp", any(handle))
        .layer(Extension(service(state)))
}

// Stateless mode: every request is authenticated again, so revoking a connection takes effect at once.
fn service(state: &AppState) -> OpenMemoryService {
    let server_state = state.clone();
    StreamableHttpService::new(
        move || Ok(OpenMemoryServer::new(server_state.clone())),
        Arc::new(NeverSessionManager::default()),
        config(state.public_url()),
    )
}

fn config(public_url: &PublicUrl) -> StreamableHttpServerConfig {
    let origin = public_url.origin();
    let hosts = LOOPBACK_HOSTS
        .into_iter()
        .map(str::to_owned)
        .chain(origin.host_str().map(str::to_owned));
    let allowed_origin = format!(
        "{}://{}:{}",
        origin.scheme(),
        origin.host_str().unwrap_or_default(),
        origin.port_or_known_default().unwrap_or_default()
    );
    StreamableHttpServerConfig::default()
        .with_legacy_session_mode(false)
        .with_json_response(true)
        .with_allowed_hosts(hosts)
        .with_allowed_origins([allowed_origin])
        .enforce_origin_validation()
}

async fn handle(
    Extension(service): Extension<OpenMemoryService>,
    actor: McpActor,
    mut request: Request,
) -> Response {
    request.extensions_mut().insert(actor);
    service.handle(request).await.map(Body::new)
}
