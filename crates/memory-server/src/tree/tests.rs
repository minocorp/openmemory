use std::sync::Arc;

use object_store::memory::InMemory;
use sqlx::PgPool;

use super::*;
use crate::access::Actor;
use crate::files::FileContent;
use crate::test_support::{agent, create, scoped};

async fn seed(pool: &PgPool) -> (Files, Actor) {
    let actor = agent(pool, "mino", "claude-code").await;
    let files = Files::new(pool.clone(), Arc::new(InMemory::new()), 1024);
    create(pool, &actor, "start", "Rules.\n").await;
    create(pool, &actor, "clients/acme", "one\n").await;
    create(pool, &actor, "clients/deep/beta", "two\n").await;
    create(pool, &actor, "tax/2026", "three\n").await;
    for path in ["clients/contract.txt", "tax/receipt.txt"] {
        files
            .upload(
                &actor,
                path,
                FileContent::Text("text".to_owned()),
                "text/plain",
            )
            .await
            .unwrap();
    }
    (files, actor)
}

fn counts(tree: &Tree) -> Vec<(&str, u32, u32)> {
    tree.folders
        .iter()
        .map(|folder| (folder.path.as_str(), folder.pages, folder.files))
        .collect()
}

#[sqlx::test]
async fn folders_count_everything_below_them(pool: PgPool) {
    let (files, actor) = seed(&pool).await;
    let tree = load(&pool, &files, &actor).await.unwrap();
    assert_eq!(
        counts(&tree),
        [("clients", 2, 1), ("clients/deep", 1, 0), ("tax", 1, 1)]
    );
    assert_eq!(tree.pages.len(), 4);
    assert_eq!(tree.files.len(), 2);
}

#[sqlx::test]
async fn a_scoped_actor_sees_only_its_folders_and_the_start_page(pool: PgPool) {
    let (files, actor) = seed(&pool).await;
    let tax_only = scoped(&actor, &["tax"], false);
    let tree = load(&pool, &files, &tax_only).await.unwrap();
    let pages: Vec<&str> = tree.pages.iter().map(|page| page.path.as_str()).collect();
    assert_eq!(pages, ["start", "tax/2026"]);
    let listed_files: Vec<&str> = tree.files.iter().map(|file| file.path.as_str()).collect();
    assert_eq!(listed_files, ["tax/receipt.txt"]);
    assert_eq!(counts(&tree), [("tax", 1, 1)]);
}

#[sqlx::test]
async fn an_empty_notebook_has_an_empty_tree(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude-code").await;
    let files = Files::new(pool.clone(), Arc::new(InMemory::new()), 1024);
    let tree = load(&pool, &files, &actor).await.unwrap();
    assert!(tree.folders.is_empty() && tree.pages.is_empty() && tree.files.is_empty());
}

#[test]
fn every_proper_prefix_is_a_folder() {
    assert_eq!(folders_above("a/b/c"), ["a", "a/b"]);
    assert_eq!(folders_above("top"), Vec::<String>::new());
}
