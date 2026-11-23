use memory_ops::VersionOp;
use sqlx::PgPool;

use super::{PAGE, edit};
use crate::pages::delete::delete;
use crate::pages::history::{HistoryQuery, history};
use crate::test_support::{agent, another_connection, create, path};

#[sqlx::test]
async fn history_lists_versions_newest_first_with_unified_diffs(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", PAGE).await;
    edit(
        &pool,
        &actor,
        "a",
        Some(("500,000 KRW", "450,000 KRW")),
        None,
    )
    .await
    .unwrap();
    let query = HistoryQuery {
        path: Some(&path("a")),
        ..HistoryQuery::default()
    };
    let versions = history(&pool, &actor, &query).await.unwrap().versions;
    assert_eq!(versions.len(), 2);
    assert_eq!(versions[0].version, 2);
    assert_eq!(versions[0].op, VersionOp::Edit);
    assert_eq!(versions[0].author, "mino/claude-code");
    assert!(versions[0].diff.contains("-Partner. 500,000 KRW."));
    assert!(versions[0].diff.contains("+Partner. 450,000 KRW."));
    assert!(versions[0].diff.starts_with("--- a@v1\n+++ a@v2\n"));
    assert_eq!(versions[1].op, VersionOp::Create);
    assert!(versions[1].diff.starts_with("--- /dev/null\n+++ a@v1\n"));
    assert!(versions[1].diff.contains("+Partner. 500,000 KRW."));
}

#[sqlx::test]
async fn history_filters_by_author_and_limit(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    let other = another_connection(&pool, &actor, "codex").await;
    create(&pool, &actor, "a", "one\n").await;
    create(&pool, &other, "b", "two\n").await;
    edit(&pool, &actor, "a", Some(("one", "uno")), None)
        .await
        .unwrap();
    let by_codex = HistoryQuery {
        by: Some("mino/claude-code/codex"),
        ..HistoryQuery::default()
    };
    let limited = HistoryQuery {
        limit: Some(2),
        ..HistoryQuery::default()
    };
    let everyone = HistoryQuery::default();
    assert_eq!(
        history(&pool, &actor, &by_codex)
            .await
            .unwrap()
            .versions
            .len(),
        1
    );
    assert_eq!(
        history(&pool, &actor, &limited)
            .await
            .unwrap()
            .versions
            .len(),
        2
    );
    assert_eq!(
        history(&pool, &actor, &everyone)
            .await
            .unwrap()
            .versions
            .len(),
        3
    );
}

#[sqlx::test]
async fn history_filters_by_start_date(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", "one\n").await;
    let tomorrow = time::OffsetDateTime::now_utc().date().next_day().unwrap();
    let day_before = time::OffsetDateTime::now_utc()
        .date()
        .previous_day()
        .unwrap();
    let later = HistoryQuery {
        since: Some(tomorrow),
        ..HistoryQuery::default()
    };
    let earlier = HistoryQuery {
        since: Some(day_before),
        ..HistoryQuery::default()
    };
    assert_eq!(
        history(&pool, &actor, &later).await.unwrap().versions.len(),
        0
    );
    assert_eq!(
        history(&pool, &actor, &earlier)
            .await
            .unwrap()
            .versions
            .len(),
        1
    );
}

#[sqlx::test]
async fn history_of_a_deleted_page_stays_available(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", "one\n").await;
    delete(&pool, &actor, &path("a"), None).await.unwrap();
    let query = HistoryQuery {
        path: Some(&path("a")),
        ..HistoryQuery::default()
    };
    let versions = history(&pool, &actor, &query).await.unwrap().versions;
    assert_eq!(versions[0].op, VersionOp::Delete);
    assert_eq!(versions[0].diff.len(), 0);
}
