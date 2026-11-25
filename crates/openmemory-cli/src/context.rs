use std::time::Duration;

use crate::client::Client;
use crate::credentials::{Vault, normalize_host};
use crate::error::CliError;
use crate::output::Output;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const READ_TIMEOUT: Duration = Duration::from_secs(120);

pub(crate) struct Context {
    pub(crate) output: Output,
    pub(crate) vault: Vault,
    pub(crate) http: reqwest::Client,
    pub(crate) host: Option<String>,
    pub(crate) token: Option<String>,
    pub(crate) editor: Option<String>,
    pub(crate) no_input: bool,
}

pub(crate) fn http_client() -> anyhow::Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .user_agent(concat!("openmemory-cli/", env!("CARGO_PKG_VERSION")))
        .connect_timeout(CONNECT_TIMEOUT)
        .read_timeout(READ_TIMEOUT)
        .build()?)
}

impl Context {
    pub(crate) async fn host(&self) -> anyhow::Result<String> {
        if let Some(host) = &self.host {
            return Ok(normalize_host(host)?);
        }
        self.vault
            .remembered_host()
            .await
            .ok_or_else(|| CliError::NeedsHost.into())
    }

    pub(crate) async fn client(&self) -> anyhow::Result<Client> {
        let host = self.host().await?;
        if let Some(token) = &self.token {
            return Ok(Client::environment(self.http.clone(), host, token.clone()));
        }
        let tokens = self.vault.load(&host).await?.ok_or(CliError::NeedsAuth)?;
        Ok(Client::stored(
            self.http.clone(),
            host,
            tokens,
            self.vault.clone(),
        ))
    }
}
