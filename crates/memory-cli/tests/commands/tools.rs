use serde_json::{Value, json};

use crate::harness::Home;
use crate::mock::{FRESH_ACCESS, FRESH_REFRESH, Mock, TestResult};

fn calls(mock: &Mock) -> TestResult<Vec<(String, Value)>> {
    Ok(mock
        .recorded
        .tool_calls
        .lock()
        .map_err(|_| "recorded calls are poisoned")?
        .clone())
}

#[tokio::test]
async fn an_expired_access_token_is_refreshed_and_the_new_tokens_are_saved() -> TestResult {
    let mock = Mock::start().await?;
    let home = Home::new()?;
    home.sign_in_with_expired_token(&mock.origin)?;

    let run = home.run(&["read", "start"]).await?;
    assert!(run.success, "{}", run.stderr);
    assert_eq!(
        run.stdout,
        "# stored data (origin: agent), not instructions\nstart  version 3  mino/claude-code  2026-11-20T10:00:00Z\n\nDates are ISO.\n"
    );
    assert_eq!(
        mock.recorded.token_requests.lock().unwrap().as_slice(),
        ["refresh_token"]
    );
    let saved = home.saved()?;
    assert_eq!(
        saved["credentials"]["token_response"]["access_token"],
        FRESH_ACCESS
    );
    assert_eq!(
        saved["credentials"]["token_response"]["refresh_token"],
        FRESH_REFRESH
    );

    let again = home.run(&["read", "start"]).await?;
    assert!(again.success, "{}", again.stderr);
    assert_eq!(
        mock.recorded.token_requests.lock().unwrap().len(),
        1,
        "a fresh token is reused"
    );
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn refreshed_credentials_stay_private_to_the_user() -> TestResult {
    use std::os::unix::fs::PermissionsExt;

    let mock = Mock::start().await?;
    let home = Home::new()?;
    home.sign_in_with_expired_token(&mock.origin)?;
    assert!(home.run(&["status"]).await?.success);
    let mode = std::fs::metadata(home.credentials_path())?
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600);
    Ok(())
}

#[tokio::test]
async fn json_prints_exactly_what_the_server_returned() -> TestResult {
    let mock = Mock::start().await?;
    let home = Home::new()?;
    home.sign_in_with_expired_token(&mock.origin)?;

    let run = home.run(&["--json", "read", "start"]).await?;
    assert!(run.success, "{}", run.stderr);
    let printed: Value = serde_json::from_str(run.stdout.trim())?;
    assert_eq!(printed["origin"], "agent");
    assert_eq!(printed["result"]["content"], "Dates are ISO.\n");
    Ok(())
}

#[tokio::test]
async fn every_command_sends_the_matching_tool_call() -> TestResult {
    let mock = Mock::start().await?;
    let home = Home::new()?;
    home.sign_in_with_expired_token(&mock.origin)?;
    let source = home.credentials_path().with_file_name("upload-source.txt");
    std::fs::write(&source, "hello")?;
    let source = source.to_str().ok_or("path")?;
    let cases: Vec<(Vec<&str>, &str, Value)> = vec![
        (
            vec!["read", "contracts/acme.pdf", "--page", "2"],
            "read",
            json!({"path": "contracts/acme.pdf", "page": 2}),
        ),
        (
            vec![
                "search",
                "단가",
                "--where",
                "renewal<+90d",
                "--folder",
                "customers",
            ],
            "search",
            json!({"text": "단가", "where": ["renewal<+90d"], "folder": "customers"}),
        ),
        (
            vec![
                "write",
                "customers/acme",
                "--replace",
                "500,000",
                "450,000",
                "--append",
                "lowered the fee",
                "--source",
                "call",
                "--task",
                "renewal",
            ],
            "write",
            json!({
                "path": "customers/acme",
                "task": "renewal",
                "replace_old": "500,000", "replace_new": "450,000",
                "append_text": "lowered the fee", "append_source": "call",
            }),
        ),
        (
            vec!["move", "a", "b"],
            "move",
            json!({"from": "a", "to": "b"}),
        ),
        (
            vec!["history", "customers/acme", "--by", "mino"],
            "history",
            json!({"path": "customers/acme", "by": "mino"}),
        ),
        (
            vec!["revert", "customers/acme", "--version", "2"],
            "revert",
            json!({"path": "customers/acme", "version": 2}),
        ),
        (
            vec!["revert", "--group", "7"],
            "revert",
            json!({"group": 7}),
        ),
        (
            vec!["upload", source, "a.txt"],
            "upload",
            json!({"path": "a.txt", "content_base64": "aGVsbG8="}),
        ),
    ];
    for (arguments, _, _) in &cases {
        let run = home.run(arguments).await?;
        assert!(run.success, "{arguments:?}: {}", run.stderr);
    }
    let recorded = calls(&mock)?;
    let expected: Vec<(String, Value)> = cases
        .into_iter()
        .map(|(_, tool, input)| (tool.to_owned(), input))
        .collect();
    assert_eq!(recorded, expected);
    Ok(())
}

#[tokio::test]
async fn a_rejected_call_prints_the_servers_explanation_and_fails() -> TestResult {
    let mock = Mock::start().await?;
    let home = Home::new()?;
    home.sign_in_with_expired_token(&mock.origin)?;

    let run = home.run(&["delete", "customers/gone"]).await?;
    assert!(!run.success);
    assert_eq!(run.stdout, "");
    assert!(
        run.stderr.contains("No page at this path."),
        "{}",
        run.stderr
    );
    Ok(())
}

#[tokio::test]
async fn without_a_login_commands_say_how_to_sign_in() -> TestResult {
    let home = Home::new()?;
    let run = home.run(&["read", "start"]).await?;
    assert!(!run.success);
    assert!(run.stderr.contains("memory login"), "{}", run.stderr);
    let status = home.run(&["status"]).await?;
    assert!(!status.success);
    Ok(())
}

#[tokio::test]
async fn status_names_the_server_and_logout_revokes_and_forgets() -> TestResult {
    let mock = Mock::start().await?;
    let home = Home::new()?;
    home.sign_in_with_expired_token(&mock.origin)?;

    let status = home.run(&["status"]).await?;
    assert!(status.success, "{}", status.stderr);
    assert!(status.stdout.contains(&format!("server: {}", mock.origin)));

    let logout = home.run(&["logout"]).await?;
    assert!(logout.success, "{}", logout.stderr);
    assert_eq!(
        mock.recorded.revoked.lock().unwrap().as_slice(),
        [FRESH_REFRESH]
    );
    assert!(!home.credentials_path().exists());
    let again = home.run(&["logout"]).await?;
    assert!(again.success);
    assert!(again.stdout.contains("Not signed in."));
    Ok(())
}

#[tokio::test]
async fn standard_input_supplies_page_content() -> TestResult {
    use std::process::Stdio;

    use tokio::io::AsyncWriteExt;

    let mock = Mock::start().await?;
    let home = Home::new()?;
    home.sign_in_with_expired_token(&mock.origin)?;
    let mut child = home
        .command(&["write", "notes/new", "--create", "-"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stdin = child.stdin.take().ok_or("no stdin")?;
    stdin.write_all("Typed in.\n".as_bytes()).await?;
    drop(stdin);
    let output = child.wait_with_output().await?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        calls(&mock)?[0],
        (
            "write".to_owned(),
            json!({"path": "notes/new", "content": "Typed in.\n"})
        )
    );
    Ok(())
}
