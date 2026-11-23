use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    Page,
    File,
    Folder,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct FolderEntry {
    pub name: String,
    pub kind: EntryKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct FolderListing {
    pub path: String,
    pub entries: Vec<FolderEntry>,
    pub next: Option<String>,
}
