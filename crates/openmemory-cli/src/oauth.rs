use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Context;
use serde::Deserialize;

use crate::credentials::Tokens;

pub(crate) const CLIENT_ID: &str = "openmemory-cli";
const METADATA_PATH: &str = "/.well-known/oauth-authorization-server";

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct Endpoints {
    pub(crate) issuer: String,
    pub(crate) authorization_endpoint: String,
    pub(crate) token_endpoint: String,
    pub(crate) device_authorization_endpoint: String,
    pub(crate) revocation_endpoint: String,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct OAuthBody {
    error: String,
    error_description: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum TokenError {
    #[error("the server refused the token request")]
    Rejected {
        code: String,
        description: Option<String>,
    },
    #[error("the token request failed")]
    Transport(#[from] reqwest::Error),
}

pub(crate) fn resource(host: &str) -> String {
    format!("{host}/api")
}

pub(crate) fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_secs())
}

pub(crate) async fn discover(http: &reqwest::Client, host: &str) -> anyhow::Result<Endpoints> {
    http.get(format!("{host}{METADATA_PATH}"))
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .with_context(|| format!("reach the server at {host}"))?
        .json()
        .await
        .context("read the server's sign-in metadata")
}

pub(crate) async fn request_token(
    http: &reqwest::Client,
    endpoints: &Endpoints,
    form: &[(&str, &str)],
) -> Result<Tokens, TokenError> {
    let response = http
        .post(&endpoints.token_endpoint)
        .form(form)
        .send()
        .await?;
    if response.status().is_success() {
        let issued: TokenResponse = response.json().await?;
        return Ok(Tokens {
            access_token: issued.access_token,
            refresh_token: issued.refresh_token,
            expires_at: issued.expires_in.map(|seconds| now() + seconds),
        });
    }
    let body: OAuthBody = response.json().await?;
    Err(TokenError::Rejected {
        code: body.error,
        description: body.error_description,
    })
}

pub(crate) async fn revoke(http: &reqwest::Client, endpoints: &Endpoints, token: &str) {
    let form = [("token", token), ("client_id", CLIENT_ID)];
    http.post(&endpoints.revocation_endpoint)
        .form(&form)
        .send()
        .await
        .ok();
}
