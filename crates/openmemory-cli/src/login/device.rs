use std::time::Duration;

use anyhow::Context;
use serde::Deserialize;

use crate::credentials::Tokens;
use crate::login::rejected;
use crate::oauth::{self, CLIENT_ID, Endpoints, TokenError};
use crate::output::note;

const DEVICE_GRANT: &str = "urn:ietf:params:oauth:grant-type:device_code";
const SLOW_DOWN_STEP: Duration = Duration::from_secs(5);

#[derive(Debug, Deserialize)]
struct Started {
    device_code: String,
    user_code: String,
    verification_uri: String,
    verification_uri_complete: Option<String>,
    expires_in: u64,
    interval: Option<u64>,
}

pub(super) async fn run(
    http: &reqwest::Client,
    host: &str,
    endpoints: &Endpoints,
) -> anyhow::Result<Tokens> {
    let resource = oauth::resource(host);
    let form = [("client_id", CLIENT_ID), ("resource", resource.as_str())];
    let started: Started = http
        .post(&endpoints.device_authorization_endpoint)
        .form(&form)
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .context("start the device sign-in")?
        .json()
        .await
        .context("read the device sign-in reply")?;
    let address = started
        .verification_uri_complete
        .as_deref()
        .unwrap_or(&started.verification_uri);
    note(&format!(
        "Open {address} on any device and confirm the code {}",
        started.user_code
    ));
    poll(http, endpoints, &started).await
}

async fn poll(
    http: &reqwest::Client,
    endpoints: &Endpoints,
    started: &Started,
) -> anyhow::Result<Tokens> {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(started.expires_in);
    let mut interval = Duration::from_secs(started.interval.unwrap_or(5));
    let form = [
        ("grant_type", DEVICE_GRANT),
        ("client_id", CLIENT_ID),
        ("device_code", started.device_code.as_str()),
    ];
    loop {
        tokio::time::sleep(interval).await;
        match oauth::request_token(http, endpoints, &form).await {
            Ok(tokens) => return Ok(tokens),
            Err(TokenError::Rejected { code, .. }) if code == "authorization_pending" => {}
            Err(TokenError::Rejected { code, .. }) if code == "slow_down" => {
                interval += SLOW_DOWN_STEP;
            }
            Err(TokenError::Rejected { code, .. }) if code == "expired_token" => {
                anyhow::bail!("the code expired: run `openmemory auth login --device` again");
            }
            Err(error) => return Err(rejected(error)),
        }
        if tokio::time::Instant::now() >= deadline {
            anyhow::bail!("the code expired: run `openmemory auth login --device` again");
        }
    }
}
