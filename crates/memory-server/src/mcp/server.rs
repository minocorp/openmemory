use axum::http::request::Parts;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::model::{
    DiscoverResult, Implementation, InitializeRequestParams, InitializeResult, ServerCapabilities,
    ServerConfig,
};
use rmcp::service::RequestContext;
use rmcp::{ErrorData, RoleServer, ServerHandler, tool_handler};

use crate::mcp::{actor, instructions};
use crate::state::AppState;

#[derive(Clone)]
pub(super) struct MemoryServer {
    pub(super) state: AppState,
    pub(super) tool_router: ToolRouter<Self>,
}

impl MemoryServer {
    pub(super) fn new(state: AppState) -> Self {
        Self {
            state,
            tool_router: Self::tool_router(),
        }
    }

    async fn instructions(
        &self,
        context: &RequestContext<RoleServer>,
    ) -> Result<String, ErrorData> {
        let parts = context
            .extensions
            .get::<Parts>()
            .ok_or_else(|| ErrorData::internal_error("request has no http parts", None))?;
        Ok(instructions::start_page(&self.state, actor::of(parts)?).await)
    }
}

#[expect(
    clippy::unused_async_trait_impl,
    reason = "rmcp's tool_handler generates an async list_tools without await"
)]
#[tool_handler(router = self.tool_router)]
impl ServerHandler for MemoryServer {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("memory", env!("CARGO_PKG_VERSION")))
    }

    async fn initialize(
        &self,
        request: InitializeRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<InitializeResult, ErrorData> {
        context.peer.set_peer_info(request.clone());
        let mut result = self.negotiate_initialize(&request)?;
        result.instructions = Some(self.instructions(&context).await?);
        Ok(result)
    }

    async fn discover(
        &self,
        context: RequestContext<RoleServer>,
    ) -> Result<DiscoverResult, ErrorData> {
        let mut result = DiscoverResult::from_server_info(
            self.supported_protocol_versions().into_owned(),
            self.get_info(),
        );
        result.instructions = Some(self.instructions(&context).await?);
        Ok(result)
    }
}
