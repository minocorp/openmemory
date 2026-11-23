use memory_ops::{Append, Replacement, VersionOp};
use sqlx::PgPool;

use super::PAGE;
use crate::access::{Actor, UserId};
use crate::pages::changes::list;
use crate::pages::delete::delete_page;
use crate::pages::error::PagesError;
use crate::pages::history::versions_of_page;
use crate::pages::ids::{PageId, Seq, VersionNumber};
use crate::pages::listing::summaries;
use crate::pages::read::{read_by_id, read_with_id};
use crate::pages::relocate::{move_page, move_page_by_id};
use crate::pages::revert::revert_page_version;
use crate::pages::write::{Revision, write_page};
use crate::test_support::{agent, create, path, scoped};
use crate::time_zone::TimeZone;

async fn id_of(pool: &PgPool, actor: &Actor, page_path: &str) -> PageId {
    read_with_id(pool, actor, &path(page_path), None)
        .await
        .unwrap()
        .0
}

fn replace(old: &str, new: &str) -> Revision {
    Revision::Edit {
        replace: Some(Replacement {
            old: old.to_owned(),
            new: new.to_owned(),
        }),
        append: None,
    }
}

async fn write_by_id(
    pool: &PgPool,
    actor: &Actor,
    id: PageId,
    expected: i32,
    revision: Revision,
) -> Result<memory_ops::WriteOutput, PagesError> {
    write_page(
        pool,
        &TimeZone::utc(),
        actor,
        id,
        VersionNumber(expected),
        revision,
        None,
    )
    .await
}

#[sqlx::test]
async fn writes_by_id_follow_the_page_across_moves_and_never_hit_a_new_page_at_the_old_path(
    pool: PgPool,
) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "x", PAGE).await;
    let original = id_of(&pool, &actor, "x").await;
    move_page(&pool, &actor, &path("x"), &path("y"), None)
        .await
        .unwrap();
    create(&pool, &actor, "x", "squatter\n").await;
    let squatter = id_of(&pool, &actor, "x").await;

    let written = write_by_id(&pool, &actor, original, 2, replace("Partner", "Vendor"))
        .await
        .unwrap();
    assert_eq!(written.path, "y");
    assert_eq!(written.version, 3);
    let untouched = read_by_id(&pool, &actor, squatter, None).await.unwrap();
    assert_eq!(untouched.content, "squatter\n");
    assert_eq!(untouched.version, 1);
}

#[sqlx::test]
async fn a_stale_expected_version_conflicts_with_the_current_one(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", PAGE).await;
    let id = id_of(&pool, &actor, "a").await;
    write_by_id(&pool, &actor, id, 1, replace("Partner", "Vendor"))
        .await
        .unwrap();

    let overwrite = write_by_id(
        &pool,
        &actor,
        id,
        1,
        Revision::Overwrite {
            content: "new\n".to_owned(),
        },
    )
    .await;
    assert!(matches!(
        overwrite,
        Err(PagesError::Conflict {
            current_version: VersionNumber(2)
        })
    ));
    let delete = delete_page(&pool, &actor, id, VersionNumber(1), None).await;
    assert!(matches!(delete, Err(PagesError::Conflict { .. })));
    let relocate = move_page_by_id(&pool, &actor, id, VersionNumber(1), &path("b"), None).await;
    assert!(matches!(relocate, Err(PagesError::Conflict { .. })));
    let revert =
        revert_page_version(&pool, &actor, id, VersionNumber(1), VersionNumber(1), None).await;
    assert!(matches!(revert, Err(PagesError::Conflict { .. })));
    assert_eq!(
        read_by_id(&pool, &actor, id, None).await.unwrap().version,
        2
    );
}

#[sqlx::test]
async fn appending_by_id_stamps_a_timeline_line(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", PAGE).await;
    let id = id_of(&pool, &actor, "a").await;
    let revision = Revision::Edit {
        replace: None,
        append: Some(Append {
            text: "renewed".to_owned(),
            source: Some("call".to_owned()),
            date: None,
        }),
    };
    write_by_id(&pool, &actor, id, 1, revision).await.unwrap();
    let page = read_by_id(&pool, &actor, id, None).await.unwrap();
    assert_eq!(page.timeline.len(), 2);
    assert_eq!(page.timeline[1].text, "renewed");
}

#[sqlx::test]
async fn pages_outside_the_scope_or_read_only_cannot_be_written_by_id(pool: PgPool) {
    let admin = agent(&pool, "mino", "claude-code").await;
    create(&pool, &admin, "clients/a", "one\n").await;
    create(&pool, &admin, "tax/a", "two\n").await;
    let hidden = id_of(&pool, &admin, "clients/a").await;
    let visible = id_of(&pool, &admin, "tax/a").await;

    let tax_only = scoped(&admin, &["tax"], true);
    assert!(matches!(
        write_by_id(&pool, &tax_only, hidden, 1, replace("one", "uno")).await,
        Err(PagesError::NotFound)
    ));
    assert!(matches!(
        read_by_id(&pool, &tax_only, hidden, None).await,
        Err(PagesError::NotFound)
    ));
    assert!(
        write_by_id(&pool, &tax_only, visible, 1, replace("two", "dos"))
            .await
            .is_ok()
    );

    let read_only = scoped(&admin, &["tax"], false);
    assert!(matches!(
        write_by_id(&pool, &read_only, visible, 2, replace("dos", "zwei")).await,
        Err(PagesError::ReadOnly)
    ));
    assert!(matches!(
        delete_page(&pool, &read_only, visible, VersionNumber(2), None).await,
        Err(PagesError::ReadOnly)
    ));
}

#[sqlx::test]
async fn the_start_page_cannot_be_deleted_or_moved_by_id(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "start", "Rules.\n").await;
    let id = id_of(&pool, &actor, "start").await;
    assert!(matches!(
        delete_page(&pool, &actor, id, VersionNumber(1), None).await,
        Err(PagesError::ReservedPath)
    ));
    assert!(matches!(
        move_page_by_id(&pool, &actor, id, VersionNumber(1), &path("other"), None).await,
        Err(PagesError::ReservedPath)
    ));
}

#[sqlx::test]
async fn a_deleted_page_keeps_its_id_and_can_be_restored_by_id(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", PAGE).await;
    let id = id_of(&pool, &actor, "a").await;
    delete_page(&pool, &actor, id, VersionNumber(1), None)
        .await
        .unwrap();
    assert!(matches!(
        read_by_id(&pool, &actor, id, None).await,
        Err(PagesError::NotFound)
    ));
    revert_page_version(&pool, &actor, id, VersionNumber(2), VersionNumber(1), None)
        .await
        .unwrap();
    let back = read_by_id(&pool, &actor, id, None).await.unwrap();
    assert_eq!(back.content, PAGE);
    assert_eq!(back.version, 3);
}

#[sqlx::test]
async fn versions_of_a_page_paginate_by_version_number(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", PAGE).await;
    let id = id_of(&pool, &actor, "a").await;
    write_by_id(&pool, &actor, id, 1, replace("Partner", "Vendor"))
        .await
        .unwrap();
    write_by_id(&pool, &actor, id, 2, replace("Vendor", "Supplier"))
        .await
        .unwrap();

    let first = versions_of_page(&pool, &actor, id, Some(2), None)
        .await
        .unwrap();
    let numbers: Vec<i32> = first.versions.iter().map(|v| v.version).collect();
    assert_eq!(numbers, [3, 2]);
    assert_eq!(first.next_before, Some(2));
    let second = versions_of_page(&pool, &actor, id, Some(2), first.next_before)
        .await
        .unwrap();
    assert_eq!(second.versions.len(), 1);
    assert_eq!(second.versions[0].op, VersionOp::Create);
    assert_eq!(second.next_before, None);
}

#[sqlx::test]
async fn versions_of_a_moved_page_hide_old_paths_outside_the_scope(pool: PgPool) {
    let admin = agent(&pool, "mino", "claude-code").await;
    create(&pool, &admin, "private/a", PAGE).await;
    let id = id_of(&pool, &admin, "private/a").await;
    move_page(&pool, &admin, &path("private/a"), &path("tax/a"), None)
        .await
        .unwrap();
    let tax_only = scoped(&admin, &["tax"], true);
    let seen = versions_of_page(&pool, &tax_only, id, None, None)
        .await
        .unwrap();
    let numbers: Vec<i32> = seen.versions.iter().map(|v| v.version).collect();
    assert_eq!(numbers, [2], "the create under private/ stays hidden");
}

#[sqlx::test]
async fn summaries_list_live_pages_in_scope_and_always_the_start_page(pool: PgPool) {
    let admin = agent(&pool, "mino", "claude-code").await;
    create(&pool, &admin, "start", "Rules.\n").await;
    create(&pool, &admin, "clients/a", "one\n").await;
    create(&pool, &admin, "tax/a", "two\n").await;
    create(&pool, &admin, "tax/b", "three\n").await;
    let doomed = id_of(&pool, &admin, "tax/b").await;
    delete_page(&pool, &admin, doomed, VersionNumber(1), None)
        .await
        .unwrap();

    let all = summaries(&pool, &admin).await.unwrap();
    let paths: Vec<&str> = all.iter().map(|page| page.path.as_str()).collect();
    assert_eq!(paths, ["clients/a", "start", "tax/a"]);

    let tax_only = scoped(&admin, &["tax"], true);
    let visible = summaries(&pool, &tax_only).await.unwrap();
    let paths: Vec<&str> = visible.iter().map(|page| page.path.as_str()).collect();
    assert_eq!(paths, ["start", "tax/a"]);
}

#[sqlx::test]
async fn the_home_feed_skips_my_own_direct_edits_but_not_my_agents(pool: PgPool) {
    let agent_actor = agent(&pool, "mino", "claude-code").await;
    let direct = Actor {
        connection_id: None,
        name: "mino".to_owned(),
        ..agent_actor.clone()
    };
    create(&pool, &agent_actor, "by-agent", "one\n").await;
    create(&pool, &direct, "by-hand", "two\n").await;
    let other = agent(&pool, "kim", "codex").await;
    create(&pool, &other, "by-other", "three\n").await;

    let mine = list(&pool, &direct, Seq(0), None).await.unwrap();
    assert_eq!(version_paths(&mine), ["by-agent", "by-other"]);
    let stranger = Actor {
        user_id: UserId(direct.user_id.0 + 1000),
        ..direct
    };
    let unrelated = list(&pool, &stranger, Seq(0), None).await.unwrap();
    assert_eq!(version_paths(&unrelated).len(), 3);
}

fn version_paths(changes: &crate::pages::changes::Changes) -> Vec<&str> {
    changes
        .groups
        .iter()
        .flat_map(|group| group.versions.iter().map(|version| version.path.as_str()))
        .collect()
}

#[sqlx::test]
async fn changes_carry_page_ids_and_diffs(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "a", "one\n").await;
    let id = id_of(&pool, &actor, "a").await;
    write_by_id(&pool, &actor, id, 1, replace("one", "uno"))
        .await
        .unwrap();
    let changes = list(&pool, &actor, Seq(0), None).await.unwrap();
    let versions = &changes.groups[0].versions;
    assert_eq!(versions[0].page_id, id);
    assert!(versions[0].diff.contains("+one"));
    assert!(versions[1].diff.contains("-one"));
    assert!(versions[1].diff.contains("+uno"));
}
