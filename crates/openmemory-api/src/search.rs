use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SearchResults {
    pub pages: Vec<PageHit>,
    pub files: Vec<FileHit>,
    pub next: Option<String>,
    pub no_hits: Option<NoHits>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PageHit {
    pub page_id: i64,
    pub path: String,
    pub line_number: Option<u32>,
    pub line: Option<String>,
    pub author: String,
    pub at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct FileHit {
    pub file_id: i64,
    pub path: String,
    pub page: Option<u32>,
    pub line_number: Option<u32>,
    pub line: Option<String>,
    pub author: String,
    pub at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct NoHits {
    pub query: String,
    pub fields_in_scope: Vec<String>,
    pub folders_out_of_scope: u32,
}
