use memory_ops::{BlockReason, VersionOp};
use sqlx::PgPool;

use super::{PAGE, edit};
use crate::access::Actor;
use crate::pages::delete::delete;
use crate::pages::error::PagesError;
use crate::pages::history::{HistoryQuery, history};
use crate::pages::ids::{GroupId, VersionNumber};
use crate::pages::read::read;
use crate::pages::relocate::move_page;
use crate::pages::revert::{revert_group, revert_version};
use crate::test_support::{age_groups, agent, another_connection, create, path, scoped};

async fn group_of_latest(pool: &PgPool, actor: &Actor, page_path: &str) -> GroupId {
    let query = HistoryQuery {
        path: Some(&path(page_path)),
        limit: Some(1),
        ..HistoryQuery::default()
    };
    let versions = history(pool, actor, &query).await.unwrap().versions;
    GroupId(versions[0].group)
}

#[sqlx::test]
async fn reverting_a_version_restores_content_including_an_older_timeline(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", PAGE).await;
    edit(
        &pool,
        &actor,
        "a",
        Some(("500,000", "450,000")),
        Some("lowered"),
    )
    .await
    .unwrap();
    let reverted = revert_version(&pool, &actor, &path("a"), VersionNumber(1), None)
        .await
        .unwrap();
    assert_eq!(reverted.reverted[0].version, 3);
    let page = read(&pool, &actor, &path("a"), None).await.unwrap();
    assert_eq!(page.content, PAGE);
    let query = HistoryQuery {
        path: Some(&path("a")),
        limit: Some(1),
        ..HistoryQuery::default()
    };
    let latest = &history(&pool, &actor, &query).await.unwrap().versions[0];
    assert_eq!(latest.op, VersionOp::Revert);
    assert_eq!(latest.reverts_version, Some(1));
}

#[sqlx::test]
async fn reverting_a_move_restores_the_path_and_the_inbound_links(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "clients/acme", PAGE).await;
    create(&pool, &actor, "people/kim", "Works at [[clients/acme]].\n").await;
    move_page(
        &pool,
        &actor,
        &path("clients/acme"),
        &path("customers/acme"),
        None,
    )
    .await
    .unwrap();
    let reverted = revert_version(
        &pool,
        &actor,
        &path("customers/acme"),
        VersionNumber(1),
        None,
    )
    .await
    .unwrap();
    assert_eq!(reverted.reverted.len(), 2);
    let back = read(&pool, &actor, &path("clients/acme"), None)
        .await
        .unwrap();
    let kim = read(&pool, &actor, &path("people/kim"), None)
        .await
        .unwrap();
    assert_eq!(back.content, PAGE);
    assert_eq!(kim.body, "Works at [[clients/acme]].\n");
}

#[sqlx::test]
async fn reverting_a_delete_undeletes_the_page(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", PAGE).await;
    delete(&pool, &actor, &path("a"), None).await.unwrap();
    revert_version(&pool, &actor, &path("a"), VersionNumber(1), None)
        .await
        .unwrap();
    let page = read(&pool, &actor, &path("a"), None).await.unwrap();
    assert_eq!(page.version, 3);
    assert_eq!(page.fields[0].values, ["active"]);
    let fields = sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!" FROM page_fields"#)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(fields, 2);
}

#[sqlx::test]
async fn reverting_a_move_back_into_an_occupied_path_conflicts(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "b", "movable\n").await;
    move_page(&pool, &actor, &path("b"), &path("c"), None)
        .await
        .unwrap();
    create(&pool, &actor, "b", "squatter\n").await;
    let move_back = revert_version(&pool, &actor, &path("c"), VersionNumber(1), None).await;
    assert!(matches!(move_back, Err(PagesError::PathTaken)));
}

#[sqlx::test]
async fn reverts_to_the_current_state_or_a_missing_version_fail(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", "one\n").await;
    let same = revert_version(&pool, &actor, &path("a"), VersionNumber(1), None).await;
    let missing = revert_version(&pool, &actor, &path("a"), VersionNumber(7), None).await;
    let nowhere = revert_version(&pool, &actor, &path("zzz"), VersionNumber(1), None).await;
    assert!(matches!(same, Err(PagesError::AlreadyCurrent)));
    assert!(matches!(missing, Err(PagesError::VersionNotFound)));
    assert!(matches!(nowhere, Err(PagesError::NotFound)));
}

#[sqlx::test]
async fn reverting_a_group_restores_every_page_it_touched(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "keep", "kept\n").await;
    create(&pool, &actor, "gone", "doomed\n").await;
    age_groups(&pool).await;
    create(&pool, &actor, "fresh", "new page\n").await;
    edit(&pool, &actor, "keep", Some(("kept", "changed")), None)
        .await
        .unwrap();
    delete(&pool, &actor, &path("gone"), None).await.unwrap();
    let group = group_of_latest(&pool, &actor, "keep").await;
    let output = revert_group(&pool, &actor, group, None).await.unwrap();
    assert_eq!(output.reverted.len(), 3);
    let keep = read(&pool, &actor, &path("keep"), None).await.unwrap();
    let gone = read(&pool, &actor, &path("gone"), None).await.unwrap();
    let fresh = read(&pool, &actor, &path("fresh"), None).await;
    assert_eq!(keep.content, "kept\n");
    assert_eq!(gone.content, "doomed\n");
    assert!(matches!(fresh, Err(PagesError::NotFound)));
    let revert_group_id = group_of_latest(&pool, &actor, "keep").await;
    assert_ne!(revert_group_id, group);
}

#[sqlx::test]
async fn a_group_revert_is_all_or_nothing_and_lists_blocking_pages(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    let other = another_connection(&pool, &actor, "codex").await;
    create(&pool, &actor, "a", "one\n").await;
    create(&pool, &actor, "b", "two\n").await;
    age_groups(&pool).await;
    edit(&pool, &actor, "a", Some(("one", "uno")), None)
        .await
        .unwrap();
    edit(&pool, &actor, "b", Some(("two", "dos")), None)
        .await
        .unwrap();
    let group = group_of_latest(&pool, &actor, "b").await;
    edit(&pool, &other, "b", Some(("dos", "zwei")), None)
        .await
        .unwrap();
    let result = revert_group(&pool, &actor, group, None).await;
    match result {
        Err(PagesError::RevertBlocked(blocked)) => {
            assert_eq!(blocked.len(), 1);
            assert_eq!(blocked[0].path, "b");
            assert_eq!(blocked[0].reason, BlockReason::ChangedSince);
        }
        other => panic!("unexpected {other:?}"),
    }
    let page = read(&pool, &actor, &path("a"), None).await.unwrap();
    assert_eq!(page.content, "uno\n");
    assert_eq!(page.version, 2);
}

#[sqlx::test]
async fn a_group_revert_blocked_by_an_occupied_path_names_it(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", "one\n").await;
    age_groups(&pool).await;
    delete(&pool, &actor, &path("a"), None).await.unwrap();
    let group = group_of_latest(&pool, &actor, "a").await;
    age_groups(&pool).await;
    create(&pool, &actor, "a", "squatter\n").await;
    match revert_group(&pool, &actor, group, None).await {
        Err(PagesError::RevertBlocked(blocked)) => {
            assert_eq!(blocked[0].path, "a");
            assert_eq!(blocked[0].reason, BlockReason::PathTaken);
        }
        other => panic!("unexpected {other:?}"),
    }
}

#[sqlx::test]
async fn reverting_a_group_twice_or_an_unknown_group_fails(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", "one\n").await;
    age_groups(&pool).await;
    edit(&pool, &actor, "a", Some(("one", "uno")), None)
        .await
        .unwrap();
    let group = group_of_latest(&pool, &actor, "a").await;
    revert_group(&pool, &actor, group, None).await.unwrap();
    let again = revert_group(&pool, &actor, group, None).await;
    let unknown = revert_group(&pool, &actor, GroupId(999_999), None).await;
    assert!(matches!(again, Err(PagesError::AlreadyCurrent)));
    assert!(matches!(unknown, Err(PagesError::GroupNotFound)));
}

#[sqlx::test]
async fn group_reverts_respect_the_scope(pool: PgPool) {
    let admin = agent(&pool, "mino", "claude-code").await;
    create(&pool, &admin, "tax/a", "one\n").await;
    create(&pool, &admin, "clients/b", "two\n").await;
    let group = group_of_latest(&pool, &admin, "tax/a").await;
    let blind = scoped(&admin, &["people"], true);
    let partial = scoped(&admin, &["tax"], true);
    let read_only = scoped(&admin, &["tax", "clients"], false);
    assert!(matches!(
        revert_group(&pool, &blind, group, None).await,
        Err(PagesError::GroupNotFound)
    ));
    assert!(matches!(
        revert_group(&pool, &partial, group, None).await,
        Err(PagesError::OutOfScope)
    ));
    assert!(matches!(
        revert_group(&pool, &read_only, group, None).await,
        Err(PagesError::ReadOnly)
    ));
    assert!(read(&pool, &admin, &path("clients/b"), None).await.is_ok());
}

#[sqlx::test]
async fn a_group_revert_racing_an_edit_never_loses_the_edit(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    let other = another_connection(&pool, &actor, "codex").await;
    create(&pool, &actor, "a", "one\n").await;
    age_groups(&pool).await;
    edit(&pool, &actor, "a", Some(("one", "uno")), None)
        .await
        .unwrap();
    let group = group_of_latest(&pool, &actor, "a").await;
    let revert = revert_group(&pool, &actor, group, None);
    let concurrent = edit(&pool, &other, "a", Some(("uno", "eins")), None);
    let (revert, concurrent) = tokio::join!(revert, concurrent);
    let page = read(&pool, &actor, &path("a"), None).await.unwrap();
    match (revert, concurrent) {
        (Err(PagesError::RevertBlocked(_)), Ok(_)) => assert_eq!(page.content, "eins\n"),
        (Ok(_), Err(PagesError::Replace(_))) => assert_eq!(page.content, "one\n"),
        (revert, concurrent) => panic!("unexpected {revert:?} {concurrent:?}"),
    }
}
