use std::collections::BTreeMap;
use std::sync::Mutex;

use super::*;

#[derive(Default)]
struct Memory(Mutex<BTreeMap<String, String>>);

impl SecretStore for Memory {
    fn read(&self, account: &str) -> Result<Option<String>, StoreError> {
        Ok(self.0.lock().unwrap().get(account).cloned())
    }

    fn write(&self, account: &str, secret: &str) -> Result<(), StoreError> {
        self.0
            .lock()
            .unwrap()
            .insert(account.to_owned(), secret.to_owned());
        Ok(())
    }

    fn remove(&self, account: &str) -> Result<(), StoreError> {
        self.0.lock().unwrap().remove(account);
        Ok(())
    }
}

struct Broken;

impl SecretStore for Broken {
    fn read(&self, _: &str) -> Result<Option<String>, StoreError> {
        Err(StoreError::failed(std::io::Error::other("no dbus")))
    }

    fn write(&self, _: &str, _: &str) -> Result<(), StoreError> {
        Err(StoreError::failed(std::io::Error::other("no dbus")))
    }

    fn remove(&self, _: &str) -> Result<(), StoreError> {
        Err(StoreError::failed(std::io::Error::other("no dbus")))
    }
}

fn tokens(access: &str) -> Tokens {
    Tokens {
        access_token: access.to_owned(),
        refresh_token: Some("refresh".to_owned()),
        expires_at: Some(100),
    }
}

#[tokio::test]
async fn a_working_keychain_holds_the_credentials() {
    let directory = tempfile::tempdir().unwrap();
    let vault = Vault::new(Some(Arc::new(Memory::default())), directory.path());
    assert_eq!(
        vault.save("https://m", &tokens("a")).await.unwrap(),
        Saved::Keychain
    );
    assert_eq!(
        vault.load("https://m").await.unwrap().unwrap().access_token,
        "a"
    );
    assert!(!directory.path().join("credentials.json").exists());
}

#[tokio::test]
async fn a_broken_keychain_falls_back_to_a_private_file() {
    let directory = tempfile::tempdir().unwrap();
    let vault = Vault::new(Some(Arc::new(Broken)), directory.path());
    assert_eq!(
        vault.save("https://m", &tokens("a")).await.unwrap(),
        Saved::File
    );
    assert_eq!(
        vault.load("https://m").await.unwrap().unwrap().access_token,
        "a"
    );
    vault.delete("https://m").await.unwrap();
    assert!(vault.load("https://m").await.unwrap().is_none());
}

#[tokio::test]
async fn without_a_keychain_the_file_is_used() {
    let directory = tempfile::tempdir().unwrap();
    let vault = Vault::new(None, directory.path());
    vault.save("https://a", &tokens("one")).await.unwrap();
    vault.save("https://b", &tokens("two")).await.unwrap();
    assert_eq!(
        vault.load("https://a").await.unwrap().unwrap().access_token,
        "one"
    );
    assert_eq!(
        vault.load("https://b").await.unwrap().unwrap().access_token,
        "two"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn the_credentials_file_and_its_directory_are_private() {
    use std::os::unix::fs::PermissionsExt;
    let directory = tempfile::tempdir().unwrap();
    let nested = directory.path().join("memory");
    let vault = Vault::new(None, &nested);
    vault.save("https://m", &tokens("a")).await.unwrap();
    let mode = |path: &Path| std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode(&nested), 0o700);
    assert_eq!(mode(&nested.join("credentials.json")), 0o600);
}

#[tokio::test]
async fn the_last_host_is_remembered() {
    let directory = tempfile::tempdir().unwrap();
    let vault = Vault::new(None, directory.path());
    assert!(vault.remembered_host().await.is_none());
    vault.remember_host("https://m.example").await.unwrap();
    assert_eq!(
        vault.remembered_host().await.as_deref(),
        Some("https://m.example")
    );
}

#[test]
fn hosts_reduce_to_their_origin() {
    assert_eq!(
        normalize_host("https://m.example/").unwrap(),
        "https://m.example"
    );
    assert_eq!(
        normalize_host("http://127.0.0.1:8080/x").unwrap(),
        "http://127.0.0.1:8080"
    );
    assert!(normalize_host("m.example").is_err());
    assert!(normalize_host("ftp://m.example").is_err());
}

#[test]
fn tokens_never_print() {
    assert_eq!(format!("{:?}", tokens("secret")), "[redacted]");
}
