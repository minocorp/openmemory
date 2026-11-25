use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PageView {
    pub id: i64,
    pub path: String,
    pub version: i32,
    pub author: String,
    pub updated_at: String,
    pub fields: Vec<FieldView>,
    pub body: String,
    pub timeline: Vec<TimelineView>,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct FieldView {
    pub key: String,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct TimelineView {
    pub date: String,
    pub source: String,
    pub file: Option<FileSourceView>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct FileSourceView {
    pub path: String,
    pub page: Option<u32>,
    pub quote: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    Create,
    Edit,
    Move,
    Relink,
    Delete,
    Revert,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct VersionView {
    pub version: i32,
    pub path: String,
    pub op: Operation,
    pub author: String,
    pub at: String,
    pub group: i64,
    pub reverts_version: Option<i32>,
    pub diff: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct VersionList {
    pub versions: Vec<VersionView>,
    pub next: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Written {
    pub path: String,
    pub version: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Moved {
    pub id: i64,
    pub path: String,
    pub version: i32,
    pub rewritten: Vec<Written>,
    pub rewritten_out_of_scope: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Restored {
    pub reverted: Vec<Written>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PageCreate {
    pub content: String,
    pub task: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PageOverwrite {
    pub content: String,
    pub task: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ReplaceBody {
    pub old: String,
    pub new: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct AppendBody {
    pub text: String,
    pub source: Option<String>,
    pub date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PageEdit {
    pub replace: Option<ReplaceBody>,
    pub append: Option<AppendBody>,
    pub task: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct PageMove {
    pub to: String,
    pub task: Option<String>,
}
