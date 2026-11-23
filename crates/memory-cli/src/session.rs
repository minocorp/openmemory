use std::time::Duration;

use anyhow::{Context, anyhow, bail};
use rmcp::model::{CallToolRequestParams, ContentBlock};
use rmcp::service::RunningService;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::auth::{AuthError, AuthorizationManager};
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::{RoleClient, ServiceExt};
use serde::Serialize;
use serde::de::DeserializeOwned;

use crate::credentials::{CredentialsFile, ServerStore};
use crate::server::Server;

const CALL_TIMEOUT: Duration = Duration::from_secs(60);
const NOT_LOGGED_IN: &str = "not logged in: run `memory login <server-url>`";

pub(crate) struct Session {
    client: RunningService<RoleClient, ()>,
}

pub(crate) struct Reply<T> {
    pub(crate) raw: String,
    pub(crate) value: T,
}

pub(crate) async fn manager(
    file: &CredentialsFile,
    server: &Server,
) -> anyhow::Result<AuthorizationManager> {
    let mut manager = AuthorizationManager::new_allowing_private_addresses(server.mcp_url())
        .await
        .context("discover how the server signs in")?;
    manager.set_credential_store(ServerStore::new(file.clone(), server.clone()));
    Ok(manager)
}

pub(crate) async fn access_token(
    file: &CredentialsFile,
) -> anyhow::Result<(Server, AuthorizationManager, String)> {
    let saved = file.read().await?.ok_or_else(|| anyhow!(NOT_LOGGED_IN))?;
    let server = Server::from_origin(saved.server);
    let mut manager = manager(file, &server).await?;
    if !manager
        .initialize_from_store()
        .await
        .context("load saved credentials")?
    {
        bail!(NOT_LOGGED_IN);
    }
    let token = manager
        .get_access_token()
        .await
        .map_err(|error| match error {
            AuthError::AuthorizationRequired => {
                anyhow!(
                    "the saved sign-in expired or was revoked: run `memory login {}`",
                    server.origin()
                )
            }
            other => anyhow!(other).context("refresh the access token"),
        })?;
    Ok((server, manager, token))
}

impl Session {
    pub(crate) async fn open(file: &CredentialsFile) -> anyhow::Result<Self> {
        let (server, _, token) = access_token(file).await?;
        let config =
            StreamableHttpClientTransportConfig::with_uri(server.mcp_url()).auth_header(token);
        let client = ()
            .serve(StreamableHttpClientTransport::from_config(config))
            .await
            .with_context(|| format!("connect to {}", server.origin()))?;
        Ok(Self { client })
    }

    pub(crate) fn server_name(&self) -> Option<String> {
        self.client.peer_info().and_then(|info| {
            info.server_info
                .as_ref()
                .map(|server| format!("{} {}", server.name, server.version))
        })
    }

    pub(crate) async fn call<I: Serialize, O: DeserializeOwned>(
        &self,
        tool: &'static str,
        input: &I,
    ) -> anyhow::Result<Reply<O>> {
        let arguments = match serde_json::to_value(input)? {
            serde_json::Value::Object(arguments) => arguments,
            other => bail!("tool input must be an object, got {other}"),
        };
        let request = CallToolRequestParams::new(tool).with_arguments(arguments);
        let result = tokio::time::timeout(CALL_TIMEOUT, self.client.call_tool(request))
            .await
            .map_err(|_| anyhow!("the server did not answer in time"))?
            .with_context(|| format!("call {tool}"))?;
        let raw = result
            .content
            .iter()
            .filter_map(ContentBlock::as_text)
            .map(|block| block.text.as_str())
            .collect::<String>();
        if result.is_error.unwrap_or(false) {
            bail!("{raw}");
        }
        let value = serde_json::from_str(&raw).context("the server sent an unexpected reply")?;
        Ok(Reply { raw, value })
    }

    pub(crate) async fn close(mut self) {
        let _ = self.client.close().await;
    }
}
