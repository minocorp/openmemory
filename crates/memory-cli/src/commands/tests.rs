use std::path::Path;

use clap::Parser;

use crate::credentials::{Tokens, Vault};
use crate::error::{CliError, exit_code};
use crate::mock::{Mock, Seen};
use crate::oauth::now;
use crate::output::Output;

use super::*;

fn context(mock: &Mock, directory: &Path, token: Option<&str>) -> Context {
    Context {
        output: Output::resolve(true, false, true),
        vault: Vault::new(None, directory),
        http: reqwest::Client::new(),
        host: Some(mock.host.clone()),
        token: token.map(str::to_owned),
        editor: None,
        no_input: false,
    }
}

async fn invoke(context: &Context, arguments: &[&str]) -> anyhow::Result<()> {
    let cli =
        Cli::try_parse_from(std::iter::once("memory").chain(arguments.iter().copied())).unwrap();
    run(cli.command, context).await
}

async fn signed_in() -> (Mock, Context, tempfile::TempDir) {
    let mock = Mock::start().await;
    mock.allow("pat");
    let directory = tempfile::tempdir().unwrap();
    let context = context(&mock, directory.path(), Some("pat"));
    (mock, context, directory)
}

fn last(mock: &Mock) -> Seen {
    mock.seen().pop().unwrap()
}

fn body(seen: &Seen) -> serde_json::Value {
    serde_json::from_slice(&seen.body).unwrap()
}

#[tokio::test]
async fn viewing_a_page_reads_it_by_path() {
    let (mock, context, _directory) = signed_in().await;
    invoke(&context, &["page", "view", "clients/acme"])
        .await
        .unwrap();
    assert_eq!(last(&mock).target, "/api/pages?path=clients%2Facme");
}

#[tokio::test]
async fn a_missing_page_reports_the_problem() {
    let (_mock, context, _directory) = signed_in().await;
    let error = invoke(&context, &["page", "view", "nope"])
        .await
        .unwrap_err();
    assert_eq!(error.to_string(), "Not found (not_found)");
    assert_eq!(exit_code(&error), 1);
}

#[tokio::test]
async fn creating_a_page_sends_the_file_content() {
    let (mock, context, directory) = signed_in().await;
    let file = directory.path().join("new.md");
    std::fs::write(&file, "# New\n").unwrap();
    invoke(
        &context,
        &["page", "create", "clients/new", file.to_str().unwrap()],
    )
    .await
    .unwrap();
    let seen = last(&mock);
    assert_eq!(
        (seen.method.as_str(), seen.target.as_str()),
        ("PUT", "/api/pages?path=clients%2Fnew")
    );
    assert_eq!(body(&seen)["content"], "# New\n");
}

#[tokio::test]
async fn appending_edits_with_the_current_etag() {
    let (mock, context, _directory) = signed_in().await;
    invoke(
        &context,
        &[
            "page",
            "append",
            "clients/acme",
            "called",
            "--source",
            "phone",
            "--date",
            "2026-11-24",
        ],
    )
    .await
    .unwrap();
    let seen = last(&mock);
    assert_eq!(
        (seen.method.as_str(), seen.if_match.as_deref()),
        ("PATCH", Some("\"1\""))
    );
    assert_eq!(body(&seen)["append"]["source"], "phone");
    assert_eq!(body(&seen)["append"]["date"], "2026-11-24");
}

#[tokio::test]
async fn moving_deleting_and_reverting_resolve_the_page_first() {
    let (mock, context, _directory) = signed_in().await;
    invoke(&context, &["page", "move", "clients/acme", "clients/acme2"])
        .await
        .unwrap();
    assert_eq!(body(&last(&mock))["to"], "clients/acme2");
    invoke(&context, &["page", "delete", "clients/acme"])
        .await
        .unwrap();
    assert_eq!(last(&mock).method, "DELETE");
    invoke(&context, &["revert", "clients/acme", "1"])
        .await
        .unwrap();
    assert_eq!(last(&mock).target, "/api/pages/7/versions/1/restore");
    invoke(&context, &["history", "clients/acme"])
        .await
        .unwrap();
    assert_eq!(last(&mock).target, "/api/pages/7/versions");
}

#[tokio::test]
async fn lists_follow_cursors_and_keep_only_their_kind() {
    let (mock, context, _directory) = signed_in().await;
    invoke(&context, &["page", "list"]).await.unwrap();
    invoke(&context, &["file", "list"]).await.unwrap();
    let targets: Vec<String> = mock.seen().into_iter().map(|seen| seen.target).collect();
    assert_eq!(
        targets
            .iter()
            .filter(|target| target.contains("cursor=c2"))
            .count(),
        2
    );
}

#[tokio::test]
async fn search_sends_text_conditions_and_folder() {
    let (mock, context, _directory) = signed_in().await;
    invoke(
        &context,
        &[
            "search", "fee", "--where", "a=1", "--where", "b>2", "--folder", "clients",
        ],
    )
    .await
    .unwrap();
    let target = last(&mock).target;
    assert!(target.contains("q=fee"), "{target}");
    assert!(
        target.contains("where=a%3D1") && target.contains("where=b%3E2"),
        "{target}"
    );
    assert!(target.contains("folder=clients"), "{target}");
}

#[tokio::test]
async fn marking_changes_seen_posts_the_last_sequence() {
    let (mock, context, _directory) = signed_in().await;
    invoke(&context, &["changes", "--seen"]).await.unwrap();
    let seen = last(&mock);
    assert_eq!(seen.target, "/api/changes/seen");
    assert_eq!(body(&seen)["seq"], 5);
}

#[tokio::test]
async fn files_upload_and_download_as_streams() {
    let (mock, context, directory) = signed_in().await;
    let local = directory.path().join("a.pdf");
    std::fs::write(&local, b"%PDF-bytes").unwrap();
    invoke(
        &context,
        &["file", "upload", local.to_str().unwrap(), "scans/a.pdf"],
    )
    .await
    .unwrap();
    assert_eq!(mock.shared.lock().unwrap().uploaded, b"%PDF-bytes");
    let uploaded = mock
        .seen()
        .into_iter()
        .find(|seen| seen.method == "POST")
        .unwrap();
    assert_eq!(uploaded.content_type.as_deref(), Some("application/pdf"));
    let out = directory.path().join("down.pdf");
    invoke(
        &context,
        &[
            "file",
            "download",
            "scans/a.pdf",
            "-o",
            out.to_str().unwrap(),
        ],
    )
    .await
    .unwrap();
    assert_eq!(std::fs::read(&out).unwrap(), b"PDFDATA");
    invoke(&context, &["file", "view", "scans/a.pdf", "--page", "2"])
        .await
        .unwrap();
    assert_eq!(last(&mock).target, "/api/files/9/text?page=2");
}

#[tokio::test]
async fn api_paginate_follows_next_cursors() {
    let (mock, context, _directory) = signed_in().await;
    invoke(&context, &["api", "items", "--paginate"])
        .await
        .unwrap();
    let targets: Vec<String> = mock.seen().into_iter().map(|seen| seen.target).collect();
    assert_eq!(targets, ["/api/items", "/api/items?cursor=b"]);
}

#[tokio::test]
async fn api_fields_become_a_json_body_for_post_and_a_query_for_get() {
    let (mock, context, _directory) = signed_in().await;
    invoke(
        &context,
        &["api", "/api/changes/seen", "-f", "seq=5", "-H", "X-Test: 1"],
    )
    .await
    .unwrap();
    let seen = last(&mock);
    assert_eq!(seen.method, "POST");
    assert_eq!(body(&seen)["seq"], "5");
    invoke(
        &context,
        &["api", "folder", "-X", "GET", "-f", "path=clients", "-i"],
    )
    .await
    .unwrap();
    assert_eq!(last(&mock).target, "/api/folder?path=clients");
}

#[tokio::test]
async fn api_prints_failures_and_exits_nonzero() {
    let (_mock, context, _directory) = signed_in().await;
    let error = invoke(&context, &["api", "/api/pages", "-f", "path=nope"])
        .await
        .unwrap_err();
    assert_eq!(exit_code(&error), 1);
}

#[tokio::test]
async fn status_and_missing_credentials_map_to_the_auth_exit_code() {
    let mock = Mock::start().await;
    let directory = tempfile::tempdir().unwrap();
    let anonymous = context(&mock, directory.path(), None);
    let error = invoke(&anonymous, &["page", "view", "a"])
        .await
        .unwrap_err();
    assert_eq!(exit_code(&error), 4);
    let rejected = context(&mock, directory.path(), Some("wrong"));
    let error = invoke(&rejected, &["auth", "status"]).await.unwrap_err();
    assert!(matches!(
        error.downcast_ref::<CliError>(),
        Some(CliError::NeedsAuth)
    ));
    mock.allow("good");
    let accepted = context(&mock, directory.path(), Some("good"));
    invoke(&accepted, &["auth", "status"]).await.unwrap();
}

#[tokio::test]
async fn logout_revokes_the_refresh_token_and_forgets_the_credentials() {
    let mock = Mock::start().await;
    let directory = tempfile::tempdir().unwrap();
    let context = context(&mock, directory.path(), None);
    let tokens = Tokens {
        access_token: "a".to_owned(),
        refresh_token: Some("r".to_owned()),
        expires_at: Some(now() + 60),
    };
    context.vault.save(&mock.host, &tokens).await.unwrap();
    invoke(&context, &["auth", "logout"]).await.unwrap();
    assert_eq!(mock.shared.lock().unwrap().revoked, ["r"]);
    assert!(context.vault.load(&mock.host).await.unwrap().is_none());
}

#[tokio::test]
async fn editing_is_refused_without_input() {
    let (_mock, mut context, _directory) = signed_in().await;
    context.no_input = true;
    let error = invoke(&context, &["page", "edit", "clients/acme"])
        .await
        .unwrap_err();
    assert_eq!(exit_code(&error), 2);
}

#[cfg(unix)]
fn editor_script(directory: &Path, body: &str) -> String {
    use std::os::unix::fs::PermissionsExt;
    let script = directory.join("editor.sh");
    std::fs::write(&script, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
    script.to_str().unwrap().to_owned()
}

#[cfg(unix)]
#[tokio::test]
async fn an_edit_is_saved_with_the_etag_it_started_from() {
    let (mock, mut context, directory) = signed_in().await;
    context.editor = Some(editor_script(directory.path(), "echo more >> \"$1\""));
    invoke(&context, &["page", "edit", "clients/acme"])
        .await
        .unwrap();
    let seen = last(&mock);
    assert_eq!(
        (seen.method.as_str(), seen.if_match.as_deref()),
        ("PUT", Some("\"1\""))
    );
    assert_eq!(body(&seen)["content"], "# Acme\nmore\n");
}

#[cfg(unix)]
#[tokio::test]
async fn an_unchanged_edit_sends_nothing() {
    let (mock, mut context, directory) = signed_in().await;
    context.editor = Some(editor_script(directory.path(), "true"));
    invoke(&context, &["page", "edit", "clients/acme"])
        .await
        .unwrap();
    assert!(mock.seen().iter().all(|seen| seen.method == "GET"));
}

#[cfg(unix)]
#[tokio::test]
async fn an_edit_that_lost_a_race_explains_how_to_retry() {
    let (mock, mut context, directory) = signed_in().await;
    let marker = directory.path().join("marker");
    let script = format!("echo more >> \"$1\"\ntouch {}\nsleep 1", marker.display());
    context.editor = Some(editor_script(directory.path(), &script));
    let shared = std::sync::Arc::clone(&mock.shared);
    tokio::spawn(async move {
        while !marker.exists() {
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        shared.lock().unwrap().version += 1;
    });
    let error = invoke(&context, &["page", "edit", "clients/acme"])
        .await
        .unwrap_err();
    let message = error.to_string();
    assert!(
        message.contains("someone changed clients/acme"),
        "{message}"
    );
    assert!(
        message.contains("memory page edit clients/acme"),
        "{message}"
    );
    assert_eq!(exit_code(&error), 1);
}
