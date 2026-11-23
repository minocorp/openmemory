use memory_api::FolderListing;
use reqwest::Method;

use crate::mock::Mock;
use crate::oauth::now;

use super::*;

fn stored(mock: &Mock, vault: &Vault, access: &str, refresh: &str) -> Client {
    let tokens = Tokens {
        access_token: access.to_owned(),
        refresh_token: Some(refresh.to_owned()),
        expires_at: Some(now() + 3600),
    };
    Client::stored(
        reqwest::Client::new(),
        mock.host.clone(),
        tokens,
        vault.clone(),
    )
}

async fn probe(client: &Client) -> anyhow::Result<FolderListing> {
    client
        .json(|| {
            client
                .builder(Method::GET, "/api/folder")
                .query(&[("path", "")])
        })
        .await
}

#[tokio::test]
async fn a_rejected_token_is_refreshed_and_the_request_retried() {
    let mock = Mock::start().await;
    mock.allow_refresh("refresh-0");
    let directory = tempfile::tempdir().unwrap();
    let vault = Vault::new(None, directory.path());
    let client = stored(&mock, &vault, "expired", "refresh-0");
    vault
        .save(&mock.host, &client_tokens("expired", "refresh-0"))
        .await
        .unwrap();
    assert!(probe(&client).await.is_ok());
    let saved = vault.load(&mock.host).await.unwrap().unwrap();
    assert_eq!(saved.access_token, "access-1");
    assert_eq!(saved.refresh_token.as_deref(), Some("refresh-1"));
}

fn client_tokens(access: &str, refresh: &str) -> Tokens {
    Tokens {
        access_token: access.to_owned(),
        refresh_token: Some(refresh.to_owned()),
        expires_at: Some(now() + 3600),
    }
}

#[tokio::test]
async fn concurrent_processes_refresh_once_and_share_the_result() {
    let mock = Mock::start().await;
    mock.allow_refresh("refresh-0");
    let directory = tempfile::tempdir().unwrap();
    let vault = Vault::new(None, directory.path());
    vault
        .save(&mock.host, &client_tokens("expired", "refresh-0"))
        .await
        .unwrap();
    let first = stored(&mock, &vault, "expired", "refresh-0");
    let second = stored(&mock, &vault, "expired", "refresh-0");
    let (one, two) = tokio::join!(probe(&first), probe(&second));
    assert!(one.is_ok() && two.is_ok());
    assert_eq!(mock.shared.lock().unwrap().refresh_calls, 1);
}

#[tokio::test]
async fn a_dead_refresh_token_asks_for_a_new_sign_in() {
    let mock = Mock::start().await;
    let directory = tempfile::tempdir().unwrap();
    let vault = Vault::new(None, directory.path());
    vault
        .save(&mock.host, &client_tokens("expired", "gone"))
        .await
        .unwrap();
    let client = stored(&mock, &vault, "expired", "gone");
    let error = probe(&client).await.unwrap_err();
    assert!(matches!(
        error.downcast_ref::<CliError>(),
        Some(CliError::NeedsAuth)
    ));
}

#[tokio::test]
async fn an_environment_token_is_never_refreshed() {
    let mock = Mock::start().await;
    let client = Client::environment(reqwest::Client::new(), mock.host.clone(), "pat".to_owned());
    let error = probe(&client).await.unwrap_err();
    assert!(matches!(
        error.downcast_ref::<CliError>(),
        Some(CliError::NeedsAuth)
    ));
    assert_eq!(mock.shared.lock().unwrap().refresh_calls, 0);
}

#[tokio::test]
async fn problem_responses_become_readable_errors() {
    let mock = Mock::start().await;
    mock.allow("pat");
    let client = Client::environment(reqwest::Client::new(), mock.host.clone(), "pat".to_owned());
    let error = client
        .json::<serde_json::Value>(|| {
            client
                .builder(Method::GET, "/api/pages")
                .query(&[("path", "missing")])
        })
        .await
        .unwrap_err();
    let api = error.downcast_ref::<ApiError>().unwrap();
    assert_eq!(api.status, StatusCode::NOT_FOUND);
    assert_eq!(api.problem.as_ref().map(Problem::code), Some("not_found"));
    assert_eq!(error.to_string(), "Not found (not_found)");
}
