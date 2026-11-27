use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SearchResults {
    pub hits: Vec<SearchHit>,
    pub folders: Vec<FolderHits>,
    pub next: Option<String>,
    pub no_hits: Option<NoHits>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum HitKind {
    Page,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SearchHit {
    pub kind: HitKind,
    pub id: i64,
    pub path: String,
    pub page: Option<u32>,
    pub line_number: Option<u32>,
    pub line: Option<String>,
    pub author: String,
    pub at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct FolderHits {
    pub path: String,
    pub matches: u32,
    pub hits: Vec<SearchHit>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct NoHits {
    pub query: String,
    pub fields_in_scope: Vec<String>,
    pub folders_out_of_scope: u32,
}
