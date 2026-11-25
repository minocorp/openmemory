use std::sync::Arc;

use keyring_core::{CredentialStore, Entry, Error};

use crate::credentials::{SecretStore, StoreError};

const SERVICE: &str = "openmemory";

pub(crate) struct Keychain {
    store: Arc<CredentialStore>,
}

impl Keychain {
    pub(crate) fn system() -> Result<Self, StoreError> {
        Ok(Self {
            store: platform_store().map_err(StoreError::failed)?,
        })
    }

    fn entry(&self, account: &str) -> Result<Entry, StoreError> {
        Entry::new_in_store(&self.store, SERVICE, account).map_err(StoreError::failed)
    }
}

impl SecretStore for Keychain {
    fn read(&self, account: &str) -> Result<Option<String>, StoreError> {
        match self.entry(account)?.get_password() {
            Ok(secret) => Ok(Some(secret)),
            Err(Error::NoEntry) => Ok(None),
            Err(error) => Err(StoreError::failed(error)),
        }
    }

    fn write(&self, account: &str, secret: &str) -> Result<(), StoreError> {
        self.entry(account)?
            .set_password(secret)
            .map_err(StoreError::failed)
    }

    fn remove(&self, account: &str) -> Result<(), StoreError> {
        match self.entry(account)?.delete_credential() {
            Ok(()) | Err(Error::NoEntry) => Ok(()),
            Err(error) => Err(StoreError::failed(error)),
        }
    }
}

#[cfg(target_os = "macos")]
fn platform_store() -> keyring_core::Result<Arc<CredentialStore>> {
    Ok(apple_native_keyring_store::keychain::Store::new()?)
}

#[cfg(target_os = "linux")]
fn platform_store() -> keyring_core::Result<Arc<CredentialStore>> {
    Ok(zbus_secret_service_keyring_store::Store::new()?)
}

#[cfg(windows)]
fn platform_store() -> keyring_core::Result<Arc<CredentialStore>> {
    Ok(windows_native_keyring_store::Store::new()?)
}

#[cfg(not(any(target_os = "macos", target_os = "linux", windows)))]
fn platform_store() -> keyring_core::Result<Arc<CredentialStore>> {
    Err(Error::NotSupportedByStore(
        "no keychain on this platform".to_owned(),
    ))
}
