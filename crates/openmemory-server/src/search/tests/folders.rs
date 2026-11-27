use openmemory_ops::{HitKind, SearchInput};
use sqlx::PgPool;

use super::{add_file, all_hits, find, paths, text};
use crate::test_support::{agent, create};

#[sqlx::test]
async fn a_large_folder_does_not_hide_matches_elsewhere(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "business/tax", "정산 일정\n").await;
    for index in 0..30 {
        create(
            &pool,
            &actor,
            &format!("archive/posts/{index}"),
            "연말정산 안내\n",
        )
        .await;
    }
    let output = find(&pool, &actor, text("정산")).await;
    assert_eq!(output.hits.len(), 0);
    let folders: Vec<(&str, u32, usize)> = output
        .folders
        .iter()
        .map(|folder| (folder.path.as_str(), folder.matches, folder.hits.len()))
        .collect();
    assert_eq!(folders, [("archive", 30, 3), ("business", 1, 1)]);
    let business = output
        .folders
        .iter()
        .find(|folder| folder.path == "business")
        .unwrap();
    assert_eq!(business.hits[0].path, "business/tax");
    assert_eq!(business.hits[0].line.as_deref(), Some("정산 일정"));
}

#[sqlx::test]
async fn a_folder_from_the_result_lists_all_of_its_matches(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "business/tax", "정산 일정\n").await;
    for index in 0..5 {
        create(
            &pool,
            &actor,
            &format!("archive/posts/{index}"),
            "연말정산 안내\n",
        )
        .await;
    }
    let inside = SearchInput {
        folder: Some("archive".to_owned()),
        ..text("정산")
    };
    let output = find(&pool, &actor, inside).await;
    assert_eq!(output.folders.len(), 0);
    assert_eq!(output.hits.len(), 5);
    assert!(
        output
            .hits
            .iter()
            .all(|hit| hit.path.starts_with("archive/posts/"))
    );
}

#[sqlx::test]
async fn each_file_is_one_result_and_the_cursor_reaches_them_all(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    for index in 0..25 {
        add_file(
            &pool,
            &actor,
            &format!("scans/{index:02}.pdf"),
            &["invoice one", "invoice two", "invoice three"],
        )
        .await;
    }
    let mut seen = Vec::new();
    let mut cursor = None;
    loop {
        let output = find(
            &pool,
            &actor,
            SearchInput {
                cursor,
                ..text("invoice")
            },
        )
        .await;
        assert!(output.hits.iter().all(|hit| hit.page == Some(1)));
        seen.extend(output.hits.iter().map(|hit| hit.path.clone()));
        match output.next_cursor {
            Some(next) => cursor = Some(next),
            None => break,
        }
    }
    assert_eq!(seen.len(), 25);
    seen.dedup();
    assert_eq!(seen.len(), 25);
}

#[sqlx::test]
async fn a_file_matches_when_its_pages_hold_the_words_between_them(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    add_file(
        &pool,
        &actor,
        "contracts/lease.pdf",
        &["cover", "alpha terms", "omega terms"],
    )
    .await;
    let output = find(&pool, &actor, text("omega ALPHA")).await;
    let hits = all_hits(&output);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].page, Some(2));
    assert_eq!(hits[0].line.as_deref(), Some("alpha terms"));
    assert_eq!(
        paths(&find(&pool, &actor, text("omega lease")).await),
        ["contracts/lease.pdf"]
    );
    assert_eq!(
        all_hits(&find(&pool, &actor, text("omega missing")).await).len(),
        0
    );
}

#[sqlx::test]
async fn pages_and_files_share_one_newest_first_order(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    create(&pool, &actor, "mix/old", "shared\n").await;
    add_file(&pool, &actor, "mix/middle.pdf", &["shared"]).await;
    create(&pool, &actor, "mix/new", "shared\n").await;
    let mut seen = Vec::new();
    let mut cursor = None;
    loop {
        let input = SearchInput {
            limit: Some(1),
            cursor,
            ..text("shared")
        };
        let output = find(&pool, &actor, input).await;
        seen.extend(output.hits.iter().map(|hit| (hit.kind, hit.path.clone())));
        match output.next_cursor {
            Some(next) => cursor = Some(next),
            None => break,
        }
    }
    assert_eq!(
        seen,
        [
            (HitKind::Page, "mix/new".to_owned()),
            (HitKind::File, "mix/middle.pdf".to_owned()),
            (HitKind::Page, "mix/old".to_owned())
        ]
    );
}
