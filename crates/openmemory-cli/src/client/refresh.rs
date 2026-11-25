use std::fs::File;

use anyhow::Context;

use crate::client::{Auth, Client};
use crate::credentials::Tokens;
use crate::error::CliError;
use crate::oauth::{self, CLIENT_ID, TokenError};

const LOCK_FILE: &str = "refresh.lock";

impl Client {
    pub(super) async fn renew(&self, stale: &str) -> anyhow::Result<String> {
        let Auth::Stored { tokens, vault } = &self.auth else {
            return Err(CliError::NeedsAuth.into());
        };
        tokio::fs::create_dir_all(vault.directory())
            .await
            .context("create config directory")?;
        let _lock = lock(vault.directory().join(LOCK_FILE)).await?;
        let mut held = tokens.lock().await;
        let latest = vault
            .load(&self.host)
            .await?
            .unwrap_or_else(|| held.clone());
        if latest.access_token != stale {
            *held = latest;
            return Ok(held.access_token.clone());
        }
        let refresh_token = latest.refresh_token.clone().ok_or(CliError::NeedsAuth)?;
        let endpoints = oauth::discover(&self.http, &self.host).await?;
        let resource = oauth::resource(&self.host);
        let form = [
            ("grant_type", "refresh_token"),
            ("client_id", CLIENT_ID),
            ("refresh_token", refresh_token.as_str()),
            ("resource", resource.as_str()),
        ];
        let renewed: Tokens = match oauth::request_token(&self.http, &endpoints, &form).await {
            Ok(renewed) => renewed,
            Err(TokenError::Rejected { .. }) => return Err(CliError::NeedsAuth.into()),
            Err(TokenError::Transport(error)) => {
                return Err(anyhow::Error::new(error).context("refresh the sign-in"));
            }
        };
        vault.save(&self.host, &renewed).await?;
        *held = renewed;
        Ok(held.access_token.clone())
    }
}

async fn lock(path: std::path::PathBuf) -> anyhow::Result<File> {
    tokio::task::spawn_blocking(move || {
        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(path)?;
        file.lock()?;
        Ok::<_, std::io::Error>(file)
    })
    .await
    .context("lock task")?
    .context("lock the credentials for refresh")
}
