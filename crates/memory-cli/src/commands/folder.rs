use memory_api::{EntryKind, FolderListing};
use reqwest::Method;

use crate::context::Context;
use crate::render;

pub(super) async fn list(
    context: &Context,
    folder: Option<&str>,
    wanted: EntryKind,
) -> anyhow::Result<()> {
    let client = context.client().await?;
    let path = folder.unwrap_or_default().trim_matches('/');
    let mut entries = Vec::new();
    let mut cursor: Option<String> = None;
    loop {
        let listing: FolderListing = client
            .json(|| {
                let mut query = vec![("path", path)];
                if let Some(cursor) = &cursor {
                    query.push(("cursor", cursor));
                }
                client.builder(Method::GET, "/api/folder").query(&query)
            })
            .await?;
        entries.extend(
            listing
                .entries
                .into_iter()
                .filter(|entry| entry.kind == wanted || entry.kind == EntryKind::Folder),
        );
        cursor = listing.next;
        if cursor.is_none() {
            break;
        }
    }
    if context.output.is_json() {
        let listing = FolderListing {
            path: path.to_owned(),
            entries,
            next: None,
        };
        return context.output.json(&listing);
    }
    print!("{}", render::entries(&entries));
    Ok(())
}
