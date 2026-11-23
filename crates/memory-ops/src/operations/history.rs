use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = crate::schema::omit_null)]
pub struct HistoryInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Page path to list versions of. Leave out for changes across every page you can read."
    )]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Only versions by this author, exactly as shown in results: a person (`mino`) or a connection (`mino/claude-code`)."
    )]
    pub by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Only versions from this ISO date on, for example `2026-11-01`.")]
    pub since: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Maximum versions to return, 1 to 100. Default 20.")]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoryOutput {
    pub versions: Vec<VersionEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VersionEntry {
    pub path: String,
    pub version: i32,
    pub op: VersionOp,
    pub author: String,
    pub at: String,
    pub group: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reverts_version: Option<i32>,
    pub diff: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VersionOp {
    Create,
    Edit,
    Move,
    Relink,
    Delete,
    Revert,
}

impl VersionOp {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Edit => "edit",
            Self::Move => "move",
            Self::Relink => "relink",
            Self::Delete => "delete",
            Self::Revert => "revert",
        }
    }

    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "create" => Some(Self::Create),
            "edit" => Some(Self::Edit),
            "move" => Some(Self::Move),
            "relink" => Some(Self::Relink),
            "delete" => Some(Self::Delete),
            "revert" => Some(Self::Revert),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ops_round_trip_through_their_names() {
        for op in [
            VersionOp::Create,
            VersionOp::Edit,
            VersionOp::Move,
            VersionOp::Relink,
            VersionOp::Delete,
            VersionOp::Revert,
        ] {
            assert_eq!(VersionOp::parse(op.as_str()), Some(op));
        }
        assert_eq!(VersionOp::parse("purge"), None);
    }
}
