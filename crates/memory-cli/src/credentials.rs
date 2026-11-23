use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use rmcp::transport::auth::{
    AuthError, CredentialRefreshGuard, CredentialStore, StoredCredentials,
};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;

use crate::server::Server;

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Saved {
    pub(crate) server: String,
    pub(crate) credentials: StoredCredentials,
}

#[derive(Debug, Clone)]
pub(crate) struct CredentialsFile {
    path: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct ServerStore {
    file: CredentialsFile,
    server: Server,
}

impl CredentialsFile {
    pub(crate) fn locate() -> anyhow::Result<Self> {
        Ok(Self {
            path: config_dir()?.join("memory").join("credentials.json"),
        })
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }

    pub(crate) async fn read(&self) -> anyhow::Result<Option<Saved>> {
        match tokio::fs::read(&self.path).await {
            Ok(bytes) => Ok(Some(
                serde_json::from_slice(&bytes)
                    .context("credentials file is damaged; run `memory login` again")?,
            )),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error).context("read credentials file"),
        }
    }

    pub(crate) async fn write(&self, saved: &Saved) -> anyhow::Result<()> {
        let directory = self
            .path
            .parent()
            .context("credentials path has no directory")?;
        tokio::fs::create_dir_all(directory)
            .await
            .context("create config directory")?;
        let staged = self.path.with_extension("json.tmp");
        let mut options = tokio::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true);
        #[cfg(unix)]
        options.mode(0o600);
        let mut file = options
            .open(&staged)
            .await
            .context("write credentials file")?;
        file.write_all(&serde_json::to_vec_pretty(saved)?)
            .await
            .context("write credentials file")?;
        file.flush().await.context("write credentials file")?;
        drop(file);
        tokio::fs::rename(&staged, &self.path)
            .await
            .context("save credentials file")
    }

    pub(crate) async fn remove(&self) -> anyhow::Result<()> {
        match tokio::fs::remove_file(&self.path).await {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error).context("remove credentials file"),
        }
    }

    async fn lock(&self) -> anyhow::Result<std::fs::File> {
        let directory = self
            .path
            .parent()
            .context("credentials path has no directory")?;
        tokio::fs::create_dir_all(directory)
            .await
            .context("create config directory")?;
        let path = self.path.with_extension("lock");
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
        .context("lock credentials file")?
        .context("lock credentials file")
    }
}

impl ServerStore {
    pub(crate) fn new(file: CredentialsFile, server: Server) -> Self {
        Self { file, server }
    }
}

fn store_error(error: &anyhow::Error) -> AuthError {
    AuthError::CredentialStoreError(format!("{error:#}"))
}

#[async_trait::async_trait]
impl CredentialStore for ServerStore {
    async fn load(&self) -> Result<Option<StoredCredentials>, AuthError> {
        let saved = self
            .file
            .read()
            .await
            .map_err(|error| store_error(&error))?;
        Ok(saved.map(|saved| saved.credentials))
    }

    async fn save(&self, credentials: StoredCredentials) -> Result<(), AuthError> {
        let saved = Saved {
            server: self.server.origin().to_owned(),
            credentials,
        };
        self.file
            .write(&saved)
            .await
            .map_err(|error| store_error(&error))
    }

    async fn clear(&self) -> Result<(), AuthError> {
        self.file
            .remove()
            .await
            .map_err(|error| store_error(&error))
    }

    // Refresh tokens rotate and reuse revokes the connection, so processes must refresh one at a time.
    async fn acquire_refresh_guard(&self) -> Result<Option<CredentialRefreshGuard>, AuthError> {
        let file = self
            .file
            .lock()
            .await
            .map_err(|error| store_error(&error))?;
        Ok(Some(CredentialRefreshGuard::new(file)))
    }
}

fn config_dir() -> anyhow::Result<PathBuf> {
    #[cfg(windows)]
    {
        std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .context("APPDATA is not set")
    }
    #[cfg(not(windows))]
    {
        if let Some(dir) = std::env::var_os("XDG_CONFIG_HOME").filter(|dir| !dir.is_empty()) {
            return Ok(PathBuf::from(dir));
        }
        match std::env::var_os("HOME").filter(|home| !home.is_empty()) {
            Some(home) => Ok(PathBuf::from(home).join(".config")),
            None => bail!("HOME is not set"),
        }
    }
}

#[cfg(test)]
mod tests;
