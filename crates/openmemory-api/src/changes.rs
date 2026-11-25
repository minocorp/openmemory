use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::pages::{Operation, Written};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct Changes {
    pub since: i64,
    pub last_seq: i64,
    pub groups: Vec<ChangeGroup>,
    pub next: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ChangeGroup {
    pub id: i64,
    pub task: Option<String>,
    pub author: String,
    pub author_display_name: String,
    pub at: String,
    pub versions: Vec<ChangedVersion>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct ChangedVersion {
    pub page_id: i64,
    pub path: String,
    pub version: i32,
    pub op: Operation,
    pub seq: i64,
    pub diff: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SeenState {
    pub last_seen_seq: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct SeenRequest {
    pub seq: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct GroupReverted {
    pub reverted: Vec<Written>,
}
