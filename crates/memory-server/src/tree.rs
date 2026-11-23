use std::collections::BTreeMap;

use sqlx::PgPool;

use crate::access::Actor;
use crate::files::{FileRecord, Files, FilesError};
use crate::pages::error::PagesError;
use crate::pages::listing::{self, PageSummary};

#[derive(Debug, thiserror::Error)]
pub(crate) enum TreeError {
    #[error("page listing failed")]
    Pages(#[from] PagesError),
    #[error("file listing failed")]
    Files(#[from] FilesError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Folder {
    pub(crate) path: String,
    pub(crate) pages: u32,
    pub(crate) files: u32,
}

#[derive(Debug, Clone)]
pub(crate) struct Tree {
    pub(crate) folders: Vec<Folder>,
    pub(crate) pages: Vec<PageSummary>,
    pub(crate) files: Vec<FileRecord>,
}

pub(crate) async fn load(pool: &PgPool, files: &Files, actor: &Actor) -> Result<Tree, TreeError> {
    let pages = listing::summaries(pool, actor).await?;
    let files = files.list(actor).await?;
    Ok(Tree::new(pages, files))
}

impl Tree {
    fn new(pages: Vec<PageSummary>, files: Vec<FileRecord>) -> Self {
        let mut counts: BTreeMap<String, (u32, u32)> = BTreeMap::new();
        for page in &pages {
            for folder in folders_above(&page.path) {
                counts.entry(folder).or_default().0 += 1;
            }
        }
        for file in &files {
            for folder in folders_above(file.path.as_str()) {
                counts.entry(folder).or_default().1 += 1;
            }
        }
        Self {
            folders: counts
                .into_iter()
                .map(|(path, (pages, files))| Folder { path, pages, files })
                .collect(),
            pages,
            files,
        }
    }
}

fn folders_above(path: &str) -> Vec<String> {
    path.match_indices('/')
        .filter_map(|(index, _)| path.get(..index))
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests;
