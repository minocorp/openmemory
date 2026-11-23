use memory_ops::WriteAction;
use sqlx::PgPool;

use super::{PAGE, edit};
use crate::access::Actor;
use crate::pages::delete::delete;
use crate::pages::error::PagesError;
use crate::pages::history::{HistoryQuery, history};
use crate::pages::ids::VersionNumber;
use crate::pages::read::read;
use crate::pages::relocate::move_page;
use crate::pages::revert::revert_version;
use crate::pages::write::write;
use crate::test_support::{agent, create, path, scoped};
use crate::time_zone::TimeZone;

async fn setup(pool: &PgPool) -> (Actor, Actor) {
    let admin = agent(pool, "mino", "claude-code").await;
    create(pool, &admin, "tax/2026", PAGE).await;
    create(pool, &admin, "clients/acme", PAGE).await;
    create(pool, &admin, "start", "Folders: tax, clients\n").await;
    let tax_only = scoped(&admin, &["tax"], true);
    (admin, tax_only)
}

#[sqlx::test]
async fn pages_outside_the_scope_read_as_missing(pool: PgPool) {
    let (_, tax_only) = setup(&pool).await;
    let hidden = read(&pool, &tax_only, &path("clients/acme"), None).await;
    let visible = read(&pool, &tax_only, &path("tax/2026"), None).await;
    assert!(matches!(hidden, Err(PagesError::NotFound)));
    assert!(visible.is_ok());
}

#[sqlx::test]
async fn the_start_page_is_readable_from_any_scope_but_not_writable(pool: PgPool) {
    let (_, tax_only) = setup(&pool).await;
    let start = read(&pool, &tax_only, &path("start"), None).await.unwrap();
    assert_eq!(start.content, "Folders: tax, clients\n");
    let write_attempt = edit(&pool, &tax_only, "start", None, Some("x")).await;
    assert!(matches!(write_attempt, Err(PagesError::OutOfScope)));
}

#[sqlx::test]
async fn writes_outside_the_scope_or_without_the_toggle_are_forbidden(pool: PgPool) {
    let (admin, tax_only) = setup(&pool).await;
    let read_only = scoped(&admin, &["tax"], false);
    let create_action = || WriteAction::Create {
        content: "x".to_owned(),
    };
    let outside = write(
        &pool,
        &TimeZone::utc(),
        &tax_only,
        &path("clients/new"),
        create_action(),
        None,
    )
    .await;
    let edit_outside = edit(&pool, &tax_only, "clients/acme", None, Some("x")).await;
    let delete_outside = delete(&pool, &tax_only, &path("clients/acme"), None).await;
    let toggled = edit(&pool, &read_only, "tax/2026", None, Some("x")).await;
    assert!(matches!(outside, Err(PagesError::OutOfScope)));
    assert!(matches!(edit_outside, Err(PagesError::OutOfScope)));
    assert!(matches!(delete_outside, Err(PagesError::OutOfScope)));
    assert!(matches!(toggled, Err(PagesError::ReadOnly)));
    assert!(
        read(&pool, &read_only, &path("tax/2026"), None)
            .await
            .is_ok()
    );
}

#[sqlx::test]
async fn moves_need_write_access_on_both_sides(pool: PgPool) {
    let (_, tax_only) = setup(&pool).await;
    let out = move_page(
        &pool,
        &tax_only,
        &path("tax/2026"),
        &path("clients/x"),
        None,
    )
    .await;
    let into = move_page(
        &pool,
        &tax_only,
        &path("clients/acme"),
        &path("tax/x"),
        None,
    )
    .await;
    assert!(matches!(out, Err(PagesError::OutOfScope)));
    assert!(matches!(into, Err(PagesError::OutOfScope)));
    let inside = move_page(&pool, &tax_only, &path("tax/2026"), &path("tax/2027"), None).await;
    assert!(inside.is_ok());
}

#[sqlx::test]
async fn history_hides_pages_and_versions_outside_the_scope(pool: PgPool) {
    let (_, tax_only) = setup(&pool).await;
    let own = HistoryQuery {
        path: Some(&path("tax/2026")),
        ..HistoryQuery::default()
    };
    let hidden = HistoryQuery {
        path: Some(&path("clients/acme")),
        ..HistoryQuery::default()
    };
    assert_eq!(
        history(&pool, &tax_only, &own)
            .await
            .unwrap()
            .versions
            .len(),
        1
    );
    assert!(matches!(
        history(&pool, &tax_only, &hidden).await,
        Err(PagesError::NotFound)
    ));
    let everything = history(&pool, &tax_only, &HistoryQuery::default())
        .await
        .unwrap();
    let paths: Vec<&str> = everything
        .versions
        .iter()
        .map(|v| v.path.as_str())
        .collect();
    assert!(paths.contains(&"tax/2026"));
    assert!(!paths.contains(&"clients/acme"));
}

#[sqlx::test]
async fn old_versions_from_outside_the_scope_stay_hidden_after_a_move_in(pool: PgPool) {
    let (admin, tax_only) = setup(&pool).await;
    move_page(
        &pool,
        &admin,
        &path("clients/acme"),
        &path("tax/acme"),
        None,
    )
    .await
    .unwrap();
    let current = read(&pool, &tax_only, &path("tax/acme"), None).await;
    let before_the_move = read(&pool, &tax_only, &path("tax/acme"), Some(VersionNumber(1))).await;
    assert!(current.is_ok());
    assert!(matches!(before_the_move, Err(PagesError::VersionNotFound)));
    let reverted =
        revert_version(&pool, &tax_only, &path("tax/acme"), VersionNumber(1), None).await;
    assert!(matches!(reverted, Err(PagesError::OutOfScope)));
}

#[sqlx::test]
async fn reads_of_a_version_that_does_not_exist_fail(pool: PgPool) {
    let (admin, _) = setup(&pool).await;
    let result = read(&pool, &admin, &path("tax/2026"), Some(VersionNumber(9))).await;
    assert!(matches!(result, Err(PagesError::VersionNotFound)));
}

#[sqlx::test]
async fn the_start_page_cannot_be_moved_or_deleted(pool: PgPool) {
    let (admin, _) = setup(&pool).await;
    let moved = move_page(&pool, &admin, &path("start"), &path("home"), None).await;
    let moved_in = move_page(&pool, &admin, &path("tax/2026"), &path("start"), None).await;
    let deleted = delete(&pool, &admin, &path("start"), None).await;
    assert!(matches!(moved, Err(PagesError::ReservedPath)));
    assert!(matches!(moved_in, Err(PagesError::ReservedPath)));
    assert!(matches!(deleted, Err(PagesError::ReservedPath)));
}

#[sqlx::test]
async fn blank_or_oversized_task_labels_are_rejected(pool: PgPool) {
    let (admin, _) = setup(&pool).await;
    let action = WriteAction::Create {
        content: "x".to_owned(),
    };
    let blank = write(
        &pool,
        &TimeZone::utc(),
        &admin,
        &path("n"),
        action,
        Some(""),
    )
    .await;
    assert!(matches!(blank, Err(PagesError::TaskLength)));
}
