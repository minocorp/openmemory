mod file;
mod host;
mod keychain;

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Context;
use serde::{Deserialize, Serialize};

pub(crate) use file::FileStore;
pub(crate) use host::normalize_host;
pub(crate) use keychain::Keychain;

const LAST_HOST_FILE: &str = "host";
const FILE_NAME: &str = "credentials.json";

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct Tokens {
    pub(crate) access_token: String,
    pub(crate) refresh_token: Option<String>,
    pub(crate) expires_at: Option<u64>,
}

impl fmt::Debug for Tokens {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[redacted]")
    }
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum StoreError {
    #[error("credential store failed")]
    Failed(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl StoreError {
    pub(crate) fn failed(source: impl std::error::Error + Send + Sync + 'static) -> Self {
        Self::Failed(Box::new(source))
    }
}

pub(crate) trait SecretStore: Send + Sync {
    fn read(&self, account: &str) -> Result<Option<String>, StoreError>;
    fn write(&self, account: &str, secret: &str) -> Result<(), StoreError>;
    fn remove(&self, account: &str) -> Result<(), StoreError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Saved {
    Keychain,
    File,
}

#[derive(Clone)]
pub(crate) struct Vault {
    primary: Option<Arc<dyn SecretStore>>,
    fallback: Arc<dyn SecretStore>,
    directory: PathBuf,
}

impl Vault {
    pub(crate) fn system(directory: &Path) -> Self {
        let primary = Keychain::system()
            .ok()
            .map(|keychain| Arc::new(keychain) as Arc<_>);
        Self::new(primary, directory)
    }

    pub(crate) fn new(primary: Option<Arc<dyn SecretStore>>, directory: &Path) -> Self {
        Self {
            primary,
            fallback: Arc::new(FileStore::new(directory.join(FILE_NAME))),
            directory: directory.to_owned(),
        }
    }

    pub(crate) fn directory(&self) -> &Path {
        &self.directory
    }

    pub(crate) async fn load(&self, host: &str) -> anyhow::Result<Option<Tokens>> {
        let text = self.read(host).await?;
        text.map(|text| serde_json::from_str(&text).context("read stored credentials"))
            .transpose()
    }

    pub(crate) async fn save(&self, host: &str, tokens: &Tokens) -> anyhow::Result<Saved> {
        let text = serde_json::to_string(tokens).context("encode credentials")?;
        if let Some(primary) = self.primary.clone() {
            let account = host.to_owned();
            let secret = text.clone();
            let stored = blocking(move || primary.write(&account, &secret)).await;
            if matches!(stored, Ok(Ok(()))) {
                let fallback = Arc::clone(&self.fallback);
                let account = host.to_owned();
                blocking(move || fallback.remove(&account)).await.ok();
                return Ok(Saved::Keychain);
            }
        }
        let fallback = Arc::clone(&self.fallback);
        let account = host.to_owned();
        blocking(move || fallback.write(&account, &text))
            .await?
            .context("write credentials file")?;
        Ok(Saved::File)
    }

    pub(crate) async fn delete(&self, host: &str) -> anyhow::Result<()> {
        if let Some(primary) = self.primary.clone() {
            let account = host.to_owned();
            blocking(move || primary.remove(&account)).await.ok();
        }
        let fallback = Arc::clone(&self.fallback);
        let account = host.to_owned();
        blocking(move || fallback.remove(&account))
            .await?
            .context("remove credentials file entry")
    }

    pub(crate) async fn remember_host(&self, host: &str) -> anyhow::Result<()> {
        tokio::fs::create_dir_all(&self.directory)
            .await
            .context("create config directory")?;
        tokio::fs::write(self.directory.join(LAST_HOST_FILE), host)
            .await
            .context("remember host")
    }

    pub(crate) async fn remembered_host(&self) -> Option<String> {
        let text = tokio::fs::read_to_string(self.directory.join(LAST_HOST_FILE))
            .await
            .ok()?;
        let host = text.trim();
        (!host.is_empty()).then(|| host.to_owned())
    }

    async fn read(&self, host: &str) -> anyhow::Result<Option<String>> {
        if let Some(primary) = self.primary.clone() {
            let account = host.to_owned();
            if let Ok(Ok(Some(text))) = blocking(move || primary.read(&account)).await {
                return Ok(Some(text));
            }
        }
        let fallback = Arc::clone(&self.fallback);
        let account = host.to_owned();
        blocking(move || fallback.read(&account))
            .await?
            .context("read credentials file")
    }
}

async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> T + Send + 'static,
) -> anyhow::Result<T> {
    tokio::task::spawn_blocking(work)
        .await
        .context("credential store task")
}

pub(crate) fn config_directory() -> anyhow::Result<PathBuf> {
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .or_else(|| std::env::var_os("APPDATA"))
                .map(|home| Path::new(&home).join(".config"))
        })
        .context("no home directory: set XDG_CONFIG_HOME")?;
    Ok(base.join("memory"))
}

#[cfg(test)]
mod tests;
