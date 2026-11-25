use openmemory_ops::VersionOp;
use sqlx::PgPool;

use super::PAGE;
use crate::pages::error::PagesError;
use crate::pages::history::{HistoryQuery, history};
use crate::pages::read::read;
use crate::pages::relocate::move_page;
use crate::test_support::{agent, create, path, scoped};

const KIM: &str =
    "Works at [[clients/acme]].\n\n## Timeline\n2026-03-02 | call — met about [[clients/acme]]\n";

async fn link_target_ids(pool: &PgPool, from: &str) -> Vec<i64> {
    sqlx::query_scalar!(
        r#"SELECT l.to_page_id AS "id!"
FROM page_links AS l
JOIN pages AS p ON p.id = l.from_page_id
WHERE p.path = $1 AND l.to_page_id IS NOT NULL"#,
        from
    )
    .fetch_all(pool)
    .await
    .unwrap()
}

async fn page_id(pool: &PgPool, page_path: &str) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT id AS "id!" FROM pages WHERE path = $1 AND deleted_at IS NULL"#,
        page_path
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test]
async fn moving_rewrites_inbound_links_as_new_versions_by_the_same_actor(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "clients/acme", PAGE).await;
    create(&pool, &actor, "people/kim", KIM).await;
    let acme_id = page_id(&pool, "clients/acme").await;
    let moved = move_page(
        &pool,
        &actor,
        &path("clients/acme"),
        &path("customers/acme"),
        None,
    )
    .await
    .unwrap();
    assert_eq!((moved.path.as_str(), moved.version), ("customers/acme", 2));
    assert_eq!(moved.rewritten.len(), 1);
    assert_eq!(moved.rewritten[0].path, "people/kim");
    assert_eq!(moved.rewritten[0].version, 2);
    let kim = read(&pool, &actor, &path("people/kim"), None)
        .await
        .unwrap();
    assert!(kim.body.contains("[[customers/acme]]"));
    assert_eq!(kim.timeline[0].text, "met about [[customers/acme]]");
    assert_eq!(kim.author, "mino/claude-code (link update)");
    assert_eq!(link_target_ids(&pool, "people/kim").await, [acme_id]);
}

#[sqlx::test]
async fn inbound_rewrites_are_recorded_as_link_updates_not_edits(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "clients/acme", PAGE).await;
    create(&pool, &actor, "people/kim", KIM).await;
    move_page(
        &pool,
        &actor,
        &path("clients/acme"),
        &path("customers/acme"),
        None,
    )
    .await
    .unwrap();
    let query = HistoryQuery {
        path: Some(&path("people/kim")),
        ..HistoryQuery::default()
    };
    let versions = history(&pool, &actor, &query).await.unwrap().versions;
    assert_eq!(versions[0].op, VersionOp::Relink);
    assert_eq!(versions[0].author, "mino/claude-code (link update)");
    assert_eq!(versions[1].op, VersionOp::Create);
    assert_eq!(versions[1].author, "mino/claude-code");
    let own = HistoryQuery {
        path: Some(&path("customers/acme")),
        ..HistoryQuery::default()
    };
    let moved = history(&pool, &actor, &own).await.unwrap().versions;
    assert_eq!(moved[0].op, VersionOp::Move);
    assert_eq!(moved[0].author, "mino/claude-code");
    assert_eq!(versions[0].group, moved[0].group);
}

#[sqlx::test]
async fn moved_pages_keep_history_and_leave_the_old_path_empty(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "clients/acme", PAGE).await;
    move_page(
        &pool,
        &actor,
        &path("clients/acme"),
        &path("customers/acme"),
        None,
    )
    .await
    .unwrap();
    let old = read(&pool, &actor, &path("clients/acme"), None).await;
    assert!(matches!(old, Err(PagesError::NotFound)));
    let query = HistoryQuery {
        path: Some(&path("customers/acme")),
        ..HistoryQuery::default()
    };
    let versions = history(&pool, &actor, &query).await.unwrap().versions;
    assert_eq!(versions[0].op, VersionOp::Move);
    assert_eq!(versions[0].path, "customers/acme");
    assert_eq!(versions[1].path, "clients/acme");
    assert_eq!(versions[0].diff.len(), 0);
}

#[sqlx::test]
async fn links_written_before_their_target_existed_are_rewritten_too(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "people/kim", KIM).await;
    assert_eq!(link_target_ids(&pool, "people/kim").await.len(), 0);
    create(&pool, &actor, "clients/acme", PAGE).await;
    move_page(
        &pool,
        &actor,
        &path("clients/acme"),
        &path("customers/acme"),
        None,
    )
    .await
    .unwrap();
    let kim = read(&pool, &actor, &path("people/kim"), None)
        .await
        .unwrap();
    assert!(kim.body.contains("[[customers/acme]]"));
    assert_eq!(link_target_ids(&pool, "people/kim").await.len(), 1);
}

#[sqlx::test]
async fn self_links_and_pages_outside_the_movers_scope_are_rewritten(pool: PgPool) {
    let admin = agent(&pool, "mino", "claude-code").await;
    create(&pool, &admin, "clients/acme", "I am [[clients/acme]]\n").await;
    create(&pool, &admin, "private/diary", "visited [[clients/acme]]\n").await;
    let scoped_actor = scoped(&admin, &["clients", "customers"], true);
    let moved = move_page(
        &pool,
        &scoped_actor,
        &path("clients/acme"),
        &path("customers/acme"),
        None,
    )
    .await
    .unwrap();
    assert_eq!(moved.rewritten.len(), 0);
    assert_eq!(moved.rewritten_out_of_scope, 1);
    let own = read(&pool, &admin, &path("customers/acme"), None)
        .await
        .unwrap();
    let diary = read(&pool, &admin, &path("private/diary"), None)
        .await
        .unwrap();
    assert_eq!(own.body, "I am [[customers/acme]]\n");
    assert_eq!(diary.body, "visited [[customers/acme]]\n");
    let unreadable = read(&pool, &scoped_actor, &path("private/diary"), None).await;
    assert!(matches!(unreadable, Err(PagesError::NotFound)));
}

#[sqlx::test]
async fn move_failures_are_typed(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", "one\n").await;
    create(&pool, &actor, "b", "two\n").await;
    let taken = move_page(&pool, &actor, &path("a"), &path("b"), None).await;
    let missing = move_page(&pool, &actor, &path("nowhere"), &path("c"), None).await;
    let same = move_page(&pool, &actor, &path("a"), &path("a"), None).await;
    assert!(matches!(taken, Err(PagesError::PathTaken)));
    assert!(matches!(missing, Err(PagesError::NotFound)));
    assert!(matches!(same, Err(PagesError::SamePath)));
}

#[sqlx::test]
async fn concurrent_moves_of_one_page_let_one_win(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    let other = crate::test_support::another_connection(&pool, &actor, "codex").await;
    create(&pool, &actor, "a", "one\n").await;
    let (from, x, y) = (path("a"), path("x"), path("y"));
    let first = move_page(&pool, &actor, &from, &x, None);
    let second = move_page(&pool, &other, &from, &y, None);
    let (first, second) = tokio::join!(first, second);
    assert_eq!(
        [&first, &second]
            .iter()
            .filter(|result| result.is_ok())
            .count(),
        1
    );
}
