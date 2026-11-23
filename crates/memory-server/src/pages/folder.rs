use memory_ops::{EntryKind, FolderEntry, PagePath};
use sqlx::PgPool;

use crate::access::Actor;
use crate::pages::error::PagesError;
use crate::pages::queries::folder;
use crate::pages::scope_filter::ScopeFilter;

pub(crate) async fn entries(
    pool: &PgPool,
    actor: &Actor,
    folder_path: Option<&PagePath>,
) -> Result<Vec<FolderEntry>, PagesError> {
    let prefix = folder_path.map_or_else(String::new, |path| format!("{path}/"));
    let mut conn = pool.acquire().await?;
    let rows = folder::children(&mut conn, &prefix, &ScopeFilter::of(&actor.scope)).await?;
    Ok(rows
        .into_iter()
        .flat_map(|row| {
            let page = row.is_page.then(|| FolderEntry {
                name: row.name.clone(),
                kind: EntryKind::Page,
            });
            let folder = row.is_folder.then_some(FolderEntry {
                name: row.name,
                kind: EntryKind::Folder,
            });
            page.into_iter().chain(folder)
        })
        .collect())
}
