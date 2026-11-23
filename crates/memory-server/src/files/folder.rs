use memory_ops::{EntryKind, FolderEntry, PagePath};

use crate::access::Actor;
use crate::files::folder_queries;
use crate::files::{Files, FilesError};
use crate::pages::scope_filter::ScopeFilter;

impl Files {
    pub(crate) async fn entries(
        &self,
        actor: &Actor,
        folder: Option<&PagePath>,
    ) -> Result<Vec<FolderEntry>, FilesError> {
        let prefix = folder.map_or_else(String::new, |path| format!("{path}/"));
        let mut conn = self.pool.acquire().await?;
        let rows =
            folder_queries::children(&mut conn, &prefix, &ScopeFilter::of(&actor.scope)).await?;
        Ok(rows
            .into_iter()
            .map(|row| FolderEntry {
                name: row.name,
                kind: if row.is_file {
                    EntryKind::File
                } else {
                    EntryKind::Folder
                },
            })
            .collect())
    }
}
