mod device;
mod loopback;
mod pkce;

use std::time::Duration;

use anyhow::Context;
use url::Url;

use crate::credentials::Tokens;
use crate::error::CliError;
use crate::oauth::{self, CLIENT_ID, Endpoints, TokenError};
use crate::output::note;

const BROWSER_WAIT: Duration = Duration::from_secs(300);

pub(crate) async fn browser(
    http: &reqwest::Client,
    host: &str,
    endpoints: &Endpoints,
    open: impl FnOnce(&str),
) -> anyhow::Result<Tokens> {
    let loopback = loopback::Loopback::bind().await?;
    let redirect_uri = loopback.redirect_uri();
    let pkce = pkce::Pkce::generate();
    let state = pkce::random_text();
    let mut url =
        Url::parse(&endpoints.authorization_endpoint).context("read the authorization endpoint")?;
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", CLIENT_ID)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("code_challenge", &pkce.challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", &state)
        .append_pair("resource", &oauth::resource(host));
    note(&format!("Open this address to sign in:\n{url}"));
    open(url.as_str());
    let code = tokio::time::timeout(BROWSER_WAIT, loopback.receive(&state, &endpoints.issuer))
        .await
        .map_err(|_| anyhow::anyhow!("no sign-in arrived within five minutes"))??;
    let form = [
        ("grant_type", "authorization_code"),
        ("client_id", CLIENT_ID),
        ("code", code.as_str()),
        ("redirect_uri", redirect_uri.as_str()),
        ("code_verifier", pkce.verifier.as_str()),
        ("resource", &oauth::resource(host)),
    ];
    oauth::request_token(http, endpoints, &form)
        .await
        .map_err(rejected)
}

pub(crate) async fn device(
    http: &reqwest::Client,
    host: &str,
    endpoints: &Endpoints,
) -> anyhow::Result<Tokens> {
    device::run(http, host, endpoints).await
}

fn rejected(error: TokenError) -> anyhow::Error {
    match error {
        TokenError::Rejected { code, .. } if code == "access_denied" => CliError::Cancelled.into(),
        TokenError::Rejected { code, description } => anyhow::anyhow!(
            "the server refused the sign-in: {}",
            description.unwrap_or(code)
        ),
        TokenError::Transport(error) => {
            anyhow::Error::new(error).context("reach the token endpoint")
        }
    }
}

#[cfg(test)]
mod tests;
