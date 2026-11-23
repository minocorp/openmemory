use rmcp::transport::auth::StoredCredentials;

use super::*;

fn file_in(directory: &Path) -> CredentialsFile {
    CredentialsFile {
        path: directory.join("memory").join("credentials.json"),
    }
}

fn saved() -> Saved {
    Saved {
        server: "https://memory.example.com".to_owned(),
        credentials: StoredCredentials::new("client-1".to_owned(), None, Vec::new(), Some(7)),
    }
}

#[tokio::test]
async fn saved_credentials_read_back() {
    let directory = tempfile::tempdir().unwrap();
    let file = file_in(directory.path());
    assert!(file.read().await.unwrap().is_none());
    file.write(&saved()).await.unwrap();
    let read = file.read().await.unwrap().unwrap();
    assert_eq!(read.server, "https://memory.example.com");
    assert_eq!(read.credentials.client_id, "client-1");
}

#[cfg(unix)]
#[tokio::test]
async fn the_credentials_file_is_private_to_the_user() {
    use std::os::unix::fs::PermissionsExt;

    let directory = tempfile::tempdir().unwrap();
    let file = file_in(directory.path());
    file.write(&saved()).await.unwrap();
    file.write(&saved()).await.unwrap();
    let mode = std::fs::metadata(file.path()).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
}

#[tokio::test]
async fn a_damaged_file_asks_for_a_new_login() {
    let directory = tempfile::tempdir().unwrap();
    let file = file_in(directory.path());
    file.write(&saved()).await.unwrap();
    tokio::fs::write(file.path(), b"{not json").await.unwrap();
    let error = file.read().await.unwrap_err();
    assert!(format!("{error:#}").contains("memory login"));
}

#[tokio::test]
async fn removing_a_missing_file_is_not_an_error() {
    let directory = tempfile::tempdir().unwrap();
    let file = file_in(directory.path());
    file.remove().await.unwrap();
    file.write(&saved()).await.unwrap();
    file.remove().await.unwrap();
    assert!(file.read().await.unwrap().is_none());
}

#[tokio::test]
async fn the_store_keeps_the_server_it_was_created_for() {
    let directory = tempfile::tempdir().unwrap();
    let file = file_in(directory.path());
    let store = ServerStore::new(
        file.clone(),
        Server::parse("https://memory.example.com/mcp").unwrap(),
    );
    assert!(store.load().await.unwrap().is_none());
    store.save(saved().credentials).await.unwrap();
    assert_eq!(
        file.read().await.unwrap().unwrap().server,
        "https://memory.example.com"
    );
    assert!(store.load().await.unwrap().is_some());
    store.clear().await.unwrap();
    assert!(store.load().await.unwrap().is_none());
}

#[tokio::test]
async fn refresh_guards_exclude_each_other() {
    let directory = tempfile::tempdir().unwrap();
    let file = file_in(directory.path());
    let first = file.lock().await.unwrap();
    let second = std::thread::scope(|scope| {
        scope
            .spawn(|| {
                let other = std::fs::OpenOptions::new()
                    .write(true)
                    .open(file.path().with_extension("lock"))
                    .unwrap();
                other.try_lock().is_err()
            })
            .join()
            .unwrap()
    });
    assert!(second, "a second holder must wait");
    drop(first);
}
