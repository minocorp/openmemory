use std::time::Duration;

use reqwest::redirect::Policy;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::harness::Home;
use crate::mock::{Mock, TestResult};

const STEP_TIMEOUT: Duration = Duration::from_secs(30);

#[tokio::test]
async fn the_browser_flow_registers_follows_the_loopback_redirect_and_saves_tokens() -> TestResult {
    let mock = Mock::start().await?;
    let home = Home::new()?;
    let mut child = home.spawn(&["login", &mock.origin])?;
    let stdout = child.stdout.take().ok_or("no stdout")?;
    let mut lines = BufReader::new(stdout).lines();

    let address = tokio::time::timeout(STEP_TIMEOUT, async {
        while let Some(line) = lines.next_line().await? {
            if line.starts_with(&format!("{}/oauth/authorize", mock.origin)) {
                return Ok::<_, std::io::Error>(Some(line));
            }
        }
        Ok(None)
    })
    .await??
    .ok_or("no authorization address was printed")?;

    let client = reqwest::Client::builder()
        .redirect(Policy::none())
        .build()?;
    let authorize = client.get(&address).send().await?;
    let redirect = authorize
        .headers()
        .get("location")
        .ok_or("no redirect")?
        .to_str()?
        .to_owned();
    assert!(redirect.starts_with("http://127.0.0.1:"), "{redirect}");
    let done = client.get(&redirect).send().await?;
    assert!(done.text().await?.contains("Signed in"));

    let output = tokio::time::timeout(STEP_TIMEOUT, child.wait_with_output()).await??;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let saved = home.saved()?;
    assert_eq!(saved["server"], mock.origin);
    assert_eq!(saved["credentials"]["client_id"], "mock-client");
    assert_eq!(
        saved["credentials"]["token_response"]["access_token"],
        "access-login"
    );

    let read = home.run(&["read", "start"]).await?;
    assert!(read.success, "{}", read.stderr);
    Ok(())
}

#[tokio::test]
async fn a_refused_sign_in_reports_the_servers_reason() -> TestResult {
    let mock = Mock::start().await?;
    let home = Home::new()?;
    let mut child = home.spawn(&["login", &mock.origin])?;
    let stdout = child.stdout.take().ok_or("no stdout")?;
    let mut lines = BufReader::new(stdout).lines();
    let address = tokio::time::timeout(STEP_TIMEOUT, async {
        while let Some(line) = lines.next_line().await? {
            if line.starts_with(&format!("{}/oauth/authorize", mock.origin)) {
                return Ok::<_, std::io::Error>(Some(line));
            }
        }
        Ok(None)
    })
    .await??
    .ok_or("no authorization address was printed")?;
    let redirect_uri = url::Url::parse(&address)?
        .query_pairs()
        .find(|(key, _)| key == "redirect_uri")
        .map(|(_, value)| value.into_owned())
        .ok_or("no redirect uri")?;

    reqwest::get(format!("{redirect_uri}?error=access_denied")).await?;
    let output = tokio::time::timeout(STEP_TIMEOUT, child.wait_with_output()).await??;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("access_denied"));
    assert!(!home.credentials_path().exists());
    Ok(())
}

#[tokio::test]
async fn the_device_flow_prints_a_code_polls_and_saves_tokens() -> TestResult {
    let mock = Mock::start().await?;
    let home = Home::new()?;
    let run = home.run(&["login", &mock.origin, "--device"]).await?;
    assert!(run.success, "{}", run.stderr);
    assert!(run.stdout.contains("BCDF-GHJK"), "{}", run.stdout);
    assert!(
        run.stdout
            .contains(&format!("{}/device?user_code=BCDF-GHJK", mock.origin))
    );
    let saved = home.saved()?;
    assert_eq!(
        saved["credentials"]["token_response"]["access_token"],
        "access-device"
    );

    let read = home.run(&["read", "start"]).await?;
    assert!(read.success, "{}", read.stderr);
    Ok(())
}
