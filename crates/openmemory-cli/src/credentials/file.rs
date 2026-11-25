use std::collections::BTreeMap;
use std::io::Write;
use std::path::PathBuf;

use crate::credentials::{SecretStore, StoreError};

#[derive(Debug)]
pub(crate) struct FileStore {
    path: PathBuf,
}

impl FileStore {
    pub(crate) fn new(path: PathBuf) -> Self {
        Self { path }
    }

    fn load(&self) -> Result<BTreeMap<String, String>, StoreError> {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => serde_json::from_str(&text).map_err(StoreError::failed),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(BTreeMap::new()),
            Err(error) => Err(StoreError::failed(error)),
        }
    }

    fn store(&self, entries: &BTreeMap<String, String>) -> Result<(), StoreError> {
        let text = serde_json::to_string_pretty(entries).map_err(StoreError::failed)?;
        let directory = self.path.parent().ok_or_else(|| {
            StoreError::failed(std::io::Error::other("credentials path has no directory"))
        })?;
        create_private_directory(directory).map_err(StoreError::failed)?;
        let staging = self.path.with_extension("tmp");
        let mut file = open_private(&staging).map_err(StoreError::failed)?;
        file.write_all(text.as_bytes())
            .map_err(StoreError::failed)?;
        file.sync_all().map_err(StoreError::failed)?;
        std::fs::rename(&staging, &self.path).map_err(StoreError::failed)
    }
}

impl SecretStore for FileStore {
    fn read(&self, account: &str) -> Result<Option<String>, StoreError> {
        Ok(self.load()?.remove(account))
    }

    fn write(&self, account: &str, secret: &str) -> Result<(), StoreError> {
        let mut entries = self.load()?;
        entries.insert(account.to_owned(), secret.to_owned());
        self.store(&entries)
    }

    fn remove(&self, account: &str) -> Result<(), StoreError> {
        let mut entries = self.load()?;
        if entries.remove(account).is_none() {
            return Ok(());
        }
        self.store(&entries)
    }
}

#[cfg(unix)]
fn create_private_directory(path: &std::path::Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(path)
}

#[cfg(not(unix))]
fn create_private_directory(path: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(path)
}

#[cfg(unix)]
fn open_private(path: &std::path::Path) -> std::io::Result<std::fs::File> {
    use std::os::unix::fs::OpenOptionsExt;
    std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
}

#[cfg(not(unix))]
fn open_private(path: &std::path::Path) -> std::io::Result<std::fs::File> {
    std::fs::File::create(path)
}
