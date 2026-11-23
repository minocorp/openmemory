use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, anyhow, bail};
use reqwest::header::CONTENT_TYPE;
use rmcp::transport::auth::{CredentialStore, OAuthTokenResponse, StoredCredentials};
use serde::Deserialize;

use crate::credentials::{CredentialsFile, ServerStore};
use crate::server::Server;
use crate::session;

const CLIENT_NAME: &str = "memory CLI";
const LOOPBACK_REDIRECT: &str = "http://127.0.0.1/callback";
const DEVICE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
const SLOW_DOWN_STEP: Duration = Duration::from_secs(5);

#[derive(Debug, Deserialize)]
struct DeviceStart {
    device_code: String,
    user_code: String,
    verification_uri: String,
    verification_uri_complete: Option<String>,
    expires_in: u64,
    interval: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct TokenFailure {
    error: String,
}

pub(crate) async fn sign_in(file: &CredentialsFile, server: &Server) -> anyhow::Result<()> {
    let mut manager = session::manager(file, server).await?;
    let metadata = manager
        .resolve_metadata()
        .await
        .context("discover how the server signs in")?
        .metadata;
    manager.set_metadata(metadata.clone());
    let device_endpoint = metadata
        .additional_fields
        .get("device_authorization_endpoint")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| anyhow!("this server does not offer sign-in with a code"))?;
    let client = manager
        .register_client(CLIENT_NAME, LOOPBACK_REDIRECT, &[])
        .await
        .context("register with the server")?;
    let http = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .build()?;
    let mcp_url = server.mcp_url();
    let start: DeviceStart = serde_json::from_slice(
        &post(
            &http,
            device_endpoint,
            &[("client_id", &client.client_id), ("resource", &mcp_url)],
        )
        .await?
        .1,
    )
    .context("the server sent an unexpected device code reply")?;
    let address = start
        .verification_uri_complete
        .as_deref()
        .unwrap_or(&start.verification_uri);
    println!("Open {address}\nand confirm the code {}", start.user_code);
    let token = poll(&http, &metadata.token_endpoint, &client.client_id, &start).await?;
    let credentials =
        StoredCredentials::new(client.client_id, Some(token), Vec::new(), Some(now()))
            .with_issuer(metadata.issuer);
    ServerStore::new(file.clone(), server.clone())
        .save(credentials)
        .await
        .context("save credentials")
}

async fn poll(
    http: &reqwest::Client,
    token_endpoint: &str,
    client_id: &str,
    start: &DeviceStart,
) -> anyhow::Result<OAuthTokenResponse> {
    let mut interval = Duration::from_secs(start.interval.unwrap_or(5));
    let deadline = tokio::time::Instant::now() + Duration::from_secs(start.expires_in);
    loop {
        tokio::time::sleep(interval).await;
        if tokio::time::Instant::now() >= deadline {
            bail!("the code expired before it was confirmed");
        }
        let (status, body) = post(
            http,
            token_endpoint,
            &[
                ("grant_type", DEVICE_GRANT),
                ("device_code", &start.device_code),
                ("client_id", client_id),
            ],
        )
        .await?;
        if status.is_success() {
            return serde_json::from_slice(&body)
                .context("the server sent an unexpected token reply");
        }
        let failure: TokenFailure =
            serde_json::from_slice(&body).context("the server sent an unexpected error reply")?;
        match failure.error.as_str() {
            "authorization_pending" => {}
            "slow_down" => interval += SLOW_DOWN_STEP,
            "access_denied" => bail!("the sign-in was denied"),
            "expired_token" => bail!("the code expired before it was confirmed"),
            other => bail!("the server refused the sign-in: {other}"),
        }
    }
}

async fn post(
    http: &reqwest::Client,
    url: &str,
    fields: &[(&str, &str)],
) -> anyhow::Result<(reqwest::StatusCode, Vec<u8>)> {
    let body = url::form_urlencoded::Serializer::new(String::new())
        .extend_pairs(fields)
        .finish();
    let response = http
        .post(url)
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
        .with_context(|| format!("request {url}"))?;
    let status = response.status();
    let bytes = response.bytes().await.context("read the server reply")?;
    Ok((status, bytes.to_vec()))
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
