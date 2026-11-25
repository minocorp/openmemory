use axum::http::{Method, StatusCode};
use serde_json::json;
use sqlx::PgPool;

use crate::pages::{PAGE, created_id};
use crate::support::{App, TestResult};

#[sqlx::test]
async fn changes_show_other_authors_with_inline_diffs(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let member = app.member(&admin, "member").await?;
    created_id(&admin, "mine", "my own\n").await?;
    let id = created_id(&member, "clients/acme", PAGE).await?;
    let edit = member
        .exchange(
            Method::PATCH,
            &format!("/api/pages/{id}"),
            &[("if-match", "\"1\"")],
            Some(json!({"replace": {"old": "Partner", "new": "Vendor"}})),
        )
        .await?;
    assert_eq!(edit.status, StatusCode::OK);

    let changes = admin.get("/api/changes").await?;
    assert_eq!(changes.status, StatusCode::OK, "{:?}", changes.body);
    assert_eq!(changes.body["since"], 0);
    let groups = changes.body["groups"].as_array().ok_or("no groups")?;
    assert_eq!(groups.len(), 1, "own edits are not changes");
    assert_eq!(groups[0]["author"], "member");
    let versions = groups[0]["versions"].as_array().ok_or("no versions")?;
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0]["op"], "create");
    assert_eq!(versions[0]["page_id"], id);
    assert!(
        versions[0]["diff"]
            .as_str()
            .is_some_and(|diff| diff.contains("+Partner. 500,000 KRW."))
    );
    let diff = versions[1]["diff"].as_str().ok_or("no diff")?;
    assert!(diff.contains("-Partner. 500,000 KRW."), "{diff}");
    assert!(diff.contains("+Vendor. 500,000 KRW."), "{diff}");
    assert_eq!(
        changes.body["last_seq"], versions[1]["seq"],
        "last_seq is the newest returned version"
    );
    Ok(())
}

#[sqlx::test]
async fn marking_seen_moves_the_default_starting_point(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let member = app.member(&admin, "member").await?;
    created_id(&member, "a", "one\n").await?;
    let before = admin.get("/api/changes").await?;
    let last_seq = before.body["last_seq"].clone();
    assert_eq!(before.body["groups"].as_array().map(Vec::len), Some(1));

    let seen = admin
        .post("/api/changes/seen", json!({"seq": last_seq}))
        .await?;
    assert_eq!(seen.status, StatusCode::OK, "{:?}", seen.body);
    assert_eq!(seen.body["last_seen_seq"], last_seq);

    let after = admin.get("/api/changes").await?;
    assert_eq!(after.body["since"], last_seq);
    assert!(after.body["groups"].as_array().is_some_and(Vec::is_empty));

    let explicit = admin.get("/api/changes?since=0").await?;
    assert_eq!(explicit.body["groups"].as_array().map(Vec::len), Some(1));

    let beyond = admin
        .post("/api/changes/seen", json!({"seq": 999_999}))
        .await?;
    assert_eq!(
        beyond.body["last_seen_seq"], last_seq,
        "the marker never passes the newest version"
    );
    Ok(())
}

#[sqlx::test]
async fn reverting_a_group_restores_every_page_it_touched(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let member = app.member(&admin, "member").await?;
    let first = created_id(&admin, "a", "one\n").await?;
    let second = created_id(&member, "b", "two\n").await?;
    let change = member
        .exchange(
            Method::PATCH,
            &format!("/api/pages/{first}"),
            &[("if-match", "\"1\"")],
            Some(json!({"replace": {"old": "one", "new": "uno"}})),
        )
        .await?;
    assert_eq!(change.status, StatusCode::OK);

    let changes = admin.get("/api/changes").await?;
    let group = changes.body["groups"][0]["id"].as_i64().ok_or("no group")?;
    let reverted = admin
        .post(&format!("/api/changes/{group}/revert"), json!({}))
        .await?;
    assert_eq!(reverted.status, StatusCode::OK, "{:?}", reverted.body);
    assert_eq!(reverted.body["reverted"].as_array().map(Vec::len), Some(2));
    assert_eq!(
        admin.get(&format!("/api/pages/{first}")).await?.body["content"],
        "one\n"
    );
    assert_eq!(
        admin.get(&format!("/api/pages/{second}")).await?.status,
        StatusCode::NOT_FOUND,
        "a page the group created is deleted again"
    );
    Ok(())
}

#[sqlx::test]
async fn a_group_revert_is_blocked_when_a_page_changed_afterwards(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let member = app.member(&admin, "member").await?;
    let id = created_id(&member, "a", "one\n").await?;
    let group = admin.get("/api/changes").await?.body["groups"][0]["id"]
        .as_i64()
        .ok_or("no group")?;
    let later = admin
        .exchange(
            Method::PATCH,
            &format!("/api/pages/{id}"),
            &[("if-match", "\"1\"")],
            Some(json!({"replace": {"old": "one", "new": "uno"}})),
        )
        .await?;
    assert_eq!(later.status, StatusCode::OK);

    let blocked = admin
        .post(&format!("/api/changes/{group}/revert"), json!({}))
        .await?;
    assert_eq!(blocked.status, StatusCode::CONFLICT);
    assert_eq!(
        blocked.body["type"],
        "urn:openmemory:problem:revert_blocked"
    );
    assert_eq!(blocked.body["blocked"][0]["path"], "a");
    assert_eq!(blocked.body["blocked"][0]["reason"], "changed_since");
    assert_eq!(
        admin.get(&format!("/api/pages/{id}")).await?.body["content"],
        "uno\n"
    );

    let unknown = admin.post("/api/changes/999/revert", json!({})).await?;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);
    Ok(())
}

#[sqlx::test]
async fn change_routes_need_a_session(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let anonymous = app.browser();
    assert_eq!(
        anonymous.get("/api/changes").await?.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        anonymous
            .post("/api/changes/seen", json!({"seq": 1}))
            .await?
            .status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        anonymous
            .post("/api/changes/1/revert", json!({}))
            .await?
            .status,
        StatusCode::UNAUTHORIZED
    );
    Ok(())
}
