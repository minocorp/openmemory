use sqlx::PgPool;

use super::{PAGE, edit};
use crate::access::Actor;
use crate::pages::delete::delete;
use crate::pages::error::PagesError;
use crate::pages::ids::{GroupId, PageId};
use crate::pages::locate::locate;
use crate::pages::purge::purge;
use crate::pages::read::read_with_id;
use crate::pages::revert::revert_group;
use crate::test_support::{agent, create, path};

async fn id_of(pool: &PgPool, actor: &Actor, page_path: &str) -> PageId {
    read_with_id(pool, actor, &path(page_path), None)
        .await
        .unwrap()
        .0
}

async fn version_count(pool: &PgPool, id: PageId) -> i64 {
    sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM page_versions WHERE page_id = $1"#,
        id.0
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test]
async fn purging_removes_versions_fields_and_links_and_leaves_a_content_free_tombstone(
    pool: PgPool,
) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "clients/acme", PAGE).await;
    create(&pool, &actor, "notes/hub", "See [[clients/acme]].\n").await;
    edit(
        &pool,
        &actor,
        "clients/acme",
        Some(("Partner", "Vendor")),
        None,
    )
    .await
    .unwrap();
    let id = id_of(&pool, &actor, "clients/acme").await;
    assert_eq!(version_count(&pool, id).await, 2);

    purge(&pool, actor.user_id, id).await.unwrap();

    assert_eq!(version_count(&pool, id).await, 0);
    let pages = sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM pages WHERE id = $1"#,
        id.0
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(pages, 0);
    let fields = sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM page_fields WHERE page_id = $1"#,
        id.0
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(fields, 0);
    let links = sqlx::query_scalar!(
        r#"SELECT count(*) AS "count!" FROM page_links WHERE to_page_id = $1 OR from_page_id = $1"#,
        id.0
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(links, 0);
    let tombstone = sqlx::query!("SELECT kind, subject_id, path, user_id FROM purges")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(tombstone.kind, "page");
    assert_eq!(tombstone.subject_id, id.0);
    assert_eq!(tombstone.path, "clients/acme");
    assert_eq!(tombstone.user_id, actor.user_id.0);
    assert!(matches!(
        locate(&pool, &actor, id).await,
        Err(PagesError::Purged)
    ));
}

#[sqlx::test]
async fn purging_does_not_touch_other_pages(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", "one\n").await;
    create(&pool, &actor, "b", "two\n").await;
    let doomed = id_of(&pool, &actor, "a").await;
    let kept = id_of(&pool, &actor, "b").await;
    purge(&pool, actor.user_id, doomed).await.unwrap();
    assert_eq!(version_count(&pool, kept).await, 1);
    assert!(locate(&pool, &actor, kept).await.is_ok());
}

#[sqlx::test]
async fn a_deleted_page_can_be_purged_and_its_path_reused(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", "one\n").await;
    let id = id_of(&pool, &actor, "a").await;
    delete(&pool, &actor, &path("a"), None).await.unwrap();
    purge(&pool, actor.user_id, id).await.unwrap();
    create(&pool, &actor, "a", "again\n").await;
    assert_ne!(id_of(&pool, &actor, "a").await, id);
}

#[sqlx::test]
async fn purging_the_start_page_or_an_unknown_page_fails(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "start", "Rules.\n").await;
    let start = id_of(&pool, &actor, "start").await;
    assert!(matches!(
        purge(&pool, actor.user_id, start).await,
        Err(PagesError::ReservedPath)
    ));
    assert!(matches!(
        purge(&pool, actor.user_id, PageId(404)).await,
        Err(PagesError::NotFound)
    ));
    assert_eq!(version_count(&pool, start).await, 1);
}

#[sqlx::test]
async fn a_group_revert_skips_pages_that_were_purged(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", "one\n").await;
    create(&pool, &actor, "b", "two\n").await;
    let purged = id_of(&pool, &actor, "a").await;
    let group = sqlx::query_scalar!(
        r#"SELECT group_id AS "group_id: GroupId" FROM page_versions WHERE page_id = $1"#,
        purged.0
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    purge(&pool, actor.user_id, purged).await.unwrap();

    let output = revert_group(&pool, &actor, group, None).await.unwrap();
    assert_eq!(output.reverted.len(), 1);
    assert_eq!(output.reverted[0].path, "b");
}
