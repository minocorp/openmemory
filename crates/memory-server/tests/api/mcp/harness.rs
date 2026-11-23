use std::net::SocketAddr;

use rmcp::model::{CallToolRequestParams, ContentBlock};
use rmcp::service::RunningService;
use rmcp::transport::StreamableHttpClientTransport;
use rmcp::transport::streamable_http_client::StreamableHttpClientTransportConfig;
use rmcp::{RoleClient, ServiceExt};
use serde_json::{Value, json};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;
use url::Url;

use crate::oauth::{Grant, REDIRECT, VERIFIER, approve, consent, exchange, grant, param, register};
use crate::support::{App, Browser, TestResult};

pub(crate) struct Server {
    address: SocketAddr,
    task: JoinHandle<()>,
}

pub(crate) struct Agent {
    pub(crate) client: RunningService<RoleClient, ()>,
    pub(crate) grant: Grant,
}

#[derive(Debug)]
pub(crate) struct Outcome {
    pub(crate) is_error: bool,
    pub(crate) text: String,
}

impl Server {
    pub(crate) async fn start(app: &App) -> TestResult<Self> {
        let listener = TcpListener::bind("127.0.0.1:0").await?;
        let address = listener.local_addr()?;
        let service = app
            .router
            .clone()
            .into_make_service_with_connect_info::<SocketAddr>();
        let task = tokio::spawn(async move {
            let _ = axum::serve(listener, service).await;
        });
        Ok(Self { address, task })
    }

    pub(crate) fn url(&self) -> String {
        format!("http://{}/mcp", self.address)
    }

    pub(crate) async fn agent(&self, grant: Grant) -> TestResult<Agent> {
        let config = StreamableHttpClientTransportConfig::with_uri(self.url())
            .auth_header(grant.access.clone());
        let client = ().serve(StreamableHttpClientTransport::from_config(config)).await?;
        Ok(Agent { client, grant })
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Agent {
    pub(crate) async fn call(&self, tool: &str, arguments: Value) -> TestResult<Outcome> {
        let request = CallToolRequestParams::new(tool.to_owned())
            .with_arguments(arguments.as_object().cloned().unwrap_or_default());
        let result = self.client.call_tool(request).await?;
        let text = result
            .content
            .iter()
            .filter_map(ContentBlock::as_text)
            .map(|block| block.text.as_str())
            .collect::<String>();
        Ok(Outcome {
            is_error: result.is_error.unwrap_or(false),
            text,
        })
    }

    pub(crate) async fn ok(&self, tool: &str, arguments: Value) -> TestResult<Value> {
        let outcome = self.call(tool, arguments).await?;
        assert!(!outcome.is_error, "{tool} failed: {}", outcome.text);
        Ok(serde_json::from_str(&outcome.text)?)
    }

    pub(crate) async fn rejected(&self, tool: &str, arguments: Value) -> TestResult<String> {
        let outcome = self.call(tool, arguments).await?;
        assert!(outcome.is_error, "{tool} succeeded: {}", outcome.text);
        Ok(outcome.text)
    }
}

pub(crate) async fn grant_for(
    app: &App,
    user: &mut Browser,
    name: &str,
    scopes: &[&str],
    can_write: bool,
) -> TestResult<Grant> {
    let client_id = register(app, REDIRECT).await?;
    let reference = consent(app, &client_id).await?;
    let approved = approve(
        user,
        &reference,
        json!({"name": name, "scopes": scopes, "can_write": can_write}),
    )
    .await?;
    let target = Url::parse(
        approved.body["redirect_uri"]
            .as_str()
            .ok_or("no redirect")?,
    )?;
    let code = param(&target, "code").ok_or("no code")?;
    grant(&exchange(app, &client_id, &code, VERIFIER).await?)
}

pub(crate) async fn full_agent(
    app: &App,
    server: &Server,
    user: &mut Browser,
    name: &str,
) -> TestResult<Agent> {
    server
        .agent(grant_for(app, user, name, &["*"], true).await?)
        .await
}

pub(crate) const ACME: &str = "---\nstatus: active\nrenewal: 2027-03-01\nfee: 500000\n---\nDesign partner. 500,000 KRW per month.\n\n## Timeline\n2026-03-02 | call — contract signed\n";
