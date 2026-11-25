use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct TreeView {
    pub folders: Vec<TreeFolder>,
    pub pages: Vec<TreePage>,
    pub files: Vec<TreeFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct TreeFolder {
    pub path: String,
    pub pages: u32,
    pub files: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct TreePage {
    pub id: i64,
    pub path: String,
    pub version: i32,
    #[serde(with = "time::serde::rfc3339")]
    pub updated_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct TreeFile {
    pub id: i64,
    pub path: String,
    pub mime: String,
    pub size: i64,
}
