use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::operations::WriteOutput;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = crate::schema::omit_null)]
pub struct RevertInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(description = "Page path. Use together with `version`.")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Version number whose state to restore, from `history`. Use together with `path`."
    )]
    pub version: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Change group number from `history`: undoes every page in that group, or nothing if any of them changed since."
    )]
    pub group: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Short label for the job you are doing, 1 to 200 characters. Changes with the same label are grouped."
    )]
    pub task: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RevertTarget {
    Version { path: String, version: i32 },
    Group { group: i64 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RevertOutput {
    pub reverted: Vec<WriteOutput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockedPage {
    pub path: String,
    pub reason: BlockReason,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockReason {
    ChangedSince,
    PathTaken,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum RevertInputError {
    #[error("revert needs path with version, or group")]
    NoTarget,
    #[error("revert takes path with version, or group, not both")]
    ConflictingTargets,
}

impl RevertInput {
    pub fn target(&self) -> Result<RevertTarget, RevertInputError> {
        match (&self.path, self.version, self.group) {
            (Some(path), Some(version), None) => Ok(RevertTarget::Version {
                path: path.clone(),
                version,
            }),
            (None, None, Some(group)) => Ok(RevertTarget::Group { group }),
            (None | Some(_), None, None) | (None, Some(_), None) => Err(RevertInputError::NoTarget),
            (_, _, Some(_)) => Err(RevertInputError::ConflictingTargets),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(path: Option<&str>, version: Option<i32>, group: Option<i64>) -> RevertInput {
        RevertInput {
            path: path.map(str::to_owned),
            version,
            group,
            task: None,
        }
    }

    #[test]
    fn a_target_is_a_page_version_or_a_group() {
        let cases = [
            (
                input(Some("a"), Some(2), None),
                Ok(RevertTarget::Version {
                    path: "a".to_owned(),
                    version: 2,
                }),
            ),
            (
                input(None, None, Some(7)),
                Ok(RevertTarget::Group { group: 7 }),
            ),
            (input(None, None, None), Err(RevertInputError::NoTarget)),
            (
                input(Some("a"), None, None),
                Err(RevertInputError::NoTarget),
            ),
            (
                input(Some("a"), Some(1), Some(7)),
                Err(RevertInputError::ConflictingTargets),
            ),
        ];
        for (revert, expected) in cases {
            assert_eq!(revert.target(), expected);
        }
    }
}
