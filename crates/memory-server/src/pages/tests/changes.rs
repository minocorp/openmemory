use memory_ops::VersionOp;
use sqlx::PgPool;

use super::edit;
use crate::pages::changes::{last_seen, list, mark_seen};
use crate::pages::delete::delete;
use crate::pages::ids::Seq;
use crate::test_support::{age_groups, agent, another_connection, create, path, scoped};

#[sqlx::test]
async fn changes_are_grouped_per_group_in_order(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    let other = another_connection(&pool, &actor, "codex").await;
    create(&pool, &actor, "a", "one\n").await;
    create(&pool, &actor, "b", "two\n").await;
    create(&pool, &other, "c", "three\n").await;
    edit(&pool, &actor, "a", Some(("one", "uno")), None)
        .await
        .unwrap();
    let changes = list(&pool, &actor, Seq(0), None).await.unwrap();
    assert_eq!(changes.groups.len(), 2);
    let claude = &changes.groups[0];
    assert_eq!(claude.author, "mino/claude-code");
    let listed: Vec<(&str, i32, VersionOp)> = claude
        .versions
        .iter()
        .map(|v| (v.path.as_str(), v.version, v.op))
        .collect();
    assert_eq!(
        listed,
        [
            ("a", 1, VersionOp::Create),
            ("b", 1, VersionOp::Create),
            ("a", 2, VersionOp::Edit)
        ]
    );
    assert_eq!(changes.groups[1].author, "mino/claude-code/codex");
    assert_eq!(changes.last_seq, claude.versions[2].seq);
}

#[sqlx::test]
async fn changes_start_after_the_given_sequence(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", "one\n").await;
    let first = list(&pool, &actor, Seq(0), None).await.unwrap();
    age_groups(&pool).await;
    edit(&pool, &actor, "a", Some(("one", "uno")), None)
        .await
        .unwrap();
    let later = list(&pool, &actor, first.last_seq, None).await.unwrap();
    assert_eq!(later.groups.len(), 1);
    assert_eq!(later.groups[0].versions.len(), 1);
    let none = list(&pool, &actor, later.last_seq, None).await.unwrap();
    assert_eq!(none.groups.len(), 0);
    assert_eq!(none.last_seq, later.last_seq);
}

#[sqlx::test]
async fn the_feed_never_shows_versions_outside_the_scope(pool: PgPool) {
    let admin = agent(&pool, "mino", "claude-code").await;
    create(&pool, &admin, "tax/a", "one\n").await;
    create(&pool, &admin, "clients/b", "two\n").await;
    age_groups(&pool).await;
    create(&pool, &admin, "clients/c", "three\n").await;
    delete(&pool, &admin, &path("clients/c"), None)
        .await
        .unwrap();
    let tax_only = scoped(&admin, &["tax"], true);
    let changes = list(&pool, &tax_only, Seq(0), None).await.unwrap();
    assert_eq!(changes.groups.len(), 1);
    assert_eq!(changes.groups[0].versions.len(), 1);
    assert_eq!(changes.groups[0].versions[0].path, "tax/a");
}

#[sqlx::test]
async fn marking_seen_only_moves_forward_and_stops_at_the_latest_change(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", "one\n").await;
    edit(&pool, &actor, "a", Some(("one", "uno")), None)
        .await
        .unwrap();
    let latest = list(&pool, &actor, Seq(0), None).await.unwrap().last_seq;
    assert_eq!(last_seen(&pool, &actor).await.unwrap(), Seq(0));
    let beyond = mark_seen(&pool, &actor, Seq(latest.0 + 1000))
        .await
        .unwrap();
    assert_eq!(beyond, latest);
    let backwards = mark_seen(&pool, &actor, Seq(1)).await.unwrap();
    assert_eq!(backwards, latest);
    assert_eq!(last_seen(&pool, &actor).await.unwrap(), latest);
    let unseen = list(&pool, &actor, latest, None).await.unwrap();
    assert_eq!(unseen.groups.len(), 0);
}
