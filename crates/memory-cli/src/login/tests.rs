use crate::mock::Mock;
use crate::oauth::discover;

use super::*;

fn http() -> reqwest::Client {
    reqwest::Client::new()
}

#[tokio::test]
async fn the_browser_flow_exchanges_the_code_with_pkce() {
    let mock = Mock::start().await;
    let endpoints = discover(&http(), &mock.host).await.unwrap();
    let follower = http();
    let tokens = browser(&http(), &mock.host, &endpoints, move |url| {
        let url = url.to_owned();
        tokio::spawn(async move { follower.get(url).send().await });
    })
    .await
    .unwrap();
    assert_eq!(tokens.access_token, "access-1");
    assert_eq!(tokens.refresh_token.as_deref(), Some("refresh-1"));
    assert!(tokens.expires_at.is_some());
}

#[tokio::test]
async fn the_device_flow_polls_until_the_code_is_approved() {
    let mock = Mock::start().await;
    let endpoints = discover(&http(), &mock.host).await.unwrap();
    let tokens = device(&http(), &mock.host, &endpoints).await.unwrap();
    assert_eq!(tokens.access_token, "access-1");
    let polls = mock
        .seen()
        .iter()
        .filter(|seen| seen.target == "/oauth/token")
        .count();
    assert_eq!(polls, 3);
}
