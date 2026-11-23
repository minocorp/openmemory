use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::operations::WriteOutput;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = crate::schema::omit_null)]
pub struct MoveInput {
    #[schemars(description = "Current page path, for example `customers/acme`.")]
    pub from: String,
    #[schemars(description = "New page path. Must not exist yet.")]
    pub to: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Short label for the job you are doing, 1 to 200 characters. Changes with the same label are grouped."
    )]
    pub task: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MoveOutput {
    pub path: String,
    pub version: i32,
    pub rewritten: Vec<WriteOutput>,
    #[serde(default)]
    pub rewritten_out_of_scope: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = crate::schema::omit_null)]
pub struct DeleteInput {
    #[schemars(description = "Page path to delete, for example `customers/old-client`.")]
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Short label for the job you are doing, 1 to 200 characters. Changes with the same label are grouped."
    )]
    pub task: Option<String>,
}
