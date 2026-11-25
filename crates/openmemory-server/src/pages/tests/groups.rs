use openmemory_ops::WriteAction;
use sqlx::PgPool;

use super::edit;
use crate::pages::write::write;
use crate::test_support::{age_groups, agent, another_connection, create, path};
use crate::time_zone::TimeZone;

async fn group_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar!(r#"SELECT COUNT(*) AS "count!" FROM change_groups"#)
        .fetch_one(pool)
        .await
        .unwrap()
}

async fn tasks(pool: &PgPool) -> Vec<Option<String>> {
    sqlx::query_scalar!("SELECT task FROM change_groups ORDER BY id")
        .fetch_all(pool)
        .await
        .unwrap()
}

async fn create_with_task(
    pool: &PgPool,
    actor: &crate::access::Actor,
    name: &str,
    task: Option<&str>,
) {
    let action = WriteAction::Create {
        content: "x".to_owned(),
    };
    write(pool, &TimeZone::utc(), actor, &path(name), action, task)
        .await
        .unwrap();
}

#[sqlx::test]
async fn writes_from_one_connection_within_thirty_minutes_share_a_group(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", "x").await;
    create(&pool, &actor, "b", "x").await;
    edit(&pool, &actor, "a", Some(("x", "y")), None)
        .await
        .unwrap();
    assert_eq!(group_count(&pool).await, 1);
}

#[sqlx::test]
async fn a_gap_of_thirty_minutes_starts_a_new_group(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", "x").await;
    age_groups(&pool).await;
    create(&pool, &actor, "b", "x").await;
    assert_eq!(group_count(&pool).await, 2);
}

#[sqlx::test]
async fn different_connections_never_share_a_group(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    let other = another_connection(&pool, &actor, "codex").await;
    create(&pool, &actor, "a", "x").await;
    create(&pool, &other, "b", "x").await;
    assert_eq!(group_count(&pool).await, 2);
}

#[sqlx::test]
async fn task_labels_name_groups_and_a_new_label_starts_a_new_group(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create_with_task(&pool, &actor, "a", None).await;
    create_with_task(&pool, &actor, "b", Some("tax prep")).await;
    create_with_task(&pool, &actor, "c", Some("tax prep")).await;
    create_with_task(&pool, &actor, "d", None).await;
    create_with_task(&pool, &actor, "e", Some("client cleanup")).await;
    assert_eq!(
        tasks(&pool).await,
        [
            Some("tax prep".to_owned()),
            Some("client cleanup".to_owned())
        ]
    );
}

#[sqlx::test]
async fn simultaneous_writes_from_one_connection_still_share_one_group(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    let first = create_with_task(&pool, &actor, "a", None);
    let second = create_with_task(&pool, &actor, "b", None);
    let third = create_with_task(&pool, &actor, "c", None);
    tokio::join!(first, second, third);
    assert_eq!(group_count(&pool).await, 1);
}
