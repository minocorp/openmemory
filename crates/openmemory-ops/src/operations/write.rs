use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use time::Date;

use crate::iso_date::parse_iso_date;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = crate::schema::omit_null)]
pub struct WriteInput {
    #[schemars(description = "Page path, for example `customers/acme`.")]
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Short label for the job you are doing, 1 to 200 characters. Changes with the same label are grouped."
    )]
    pub task: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Full page content: optional YAML frontmatter, body, optional `## Timeline`. Alone it creates a new page and fails if the page exists. With `base_version` it replaces the whole page. Use without the replace and append fields."
    )]
    pub content: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "The page version you read. Only with `content`, to replace an existing page; fails if the page changed since."
    )]
    pub base_version: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Edit an existing page: exact text to find, including whitespace. Must occur exactly once in the page. Needs `replace_new`."
    )]
    pub replace_old: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Edit an existing page: text to put in place of `replace_old`. Empty text deletes it."
    )]
    pub replace_new: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Edit an existing page: one line added to the end of its timeline, the text after the dash. Write only the fact, for example `agreed to lower the fee to 450,000`. Can go with `replace_old` and `replace_new`."
    )]
    pub append_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Where the appended fact comes from: free text such as `call`, or a file path with optional page and quote such as `contracts/acme.pdf#p2 \"quoted words\"`. A quote must occur in that page of the file. Defaults to your own name. Only with `append_text`."
    )]
    pub append_source: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Date the fact happened, `YYYY-MM-DD`, not later than today. Defaults to today. Only with `append_text`."
    )]
    pub append_date: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Replacement {
    pub old: String,
    pub new: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Append {
    pub text: String,
    pub source: Option<String>,
    pub date: Option<Date>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WriteAction {
    Create {
        content: String,
    },
    Edit {
        replace: Option<Replacement>,
        append: Option<Append>,
    },
    Overwrite {
        content: String,
        base_version: i32,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WriteOutput {
    pub path: String,
    pub version: i32,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum WriteInputError {
    #[error(
        "nothing to write: give `content`, or `replace_old` with `replace_new`, or `append_text`"
    )]
    NoAction,
    #[error("`content` cannot be combined with `replace_old`, `replace_new` or `append_text`")]
    ContentWithEdits,
    #[error("`replace_old` and `replace_new` must be given together")]
    IncompleteReplace,
    #[error("`append_source` and `append_date` need `append_text`")]
    AppendWithoutText,
    #[error("`base_version` needs `content`")]
    BaseVersionWithoutContent,
    #[error("`append_date` must be a real date written `YYYY-MM-DD`")]
    InvalidDate,
}

impl WriteInput {
    pub fn action(&self) -> Result<WriteAction, WriteInputError> {
        let replace = match (&self.replace_old, &self.replace_new) {
            (Some(old), Some(new)) => Some(Replacement {
                old: old.clone(),
                new: new.clone(),
            }),
            (None, None) => None,
            (Some(_), None) | (None, Some(_)) => return Err(WriteInputError::IncompleteReplace),
        };
        if self.append_text.is_none()
            && (self.append_source.is_some() || self.append_date.is_some())
        {
            return Err(WriteInputError::AppendWithoutText);
        }
        let append = self
            .append_text
            .as_ref()
            .map(|text| {
                Ok(Append {
                    text: text.clone(),
                    source: self.append_source.clone(),
                    date: self
                        .append_date
                        .as_deref()
                        .map(|date| parse_iso_date(date).ok_or(WriteInputError::InvalidDate))
                        .transpose()?,
                })
            })
            .transpose()?;
        if self.content.is_none() && self.base_version.is_some() {
            return Err(WriteInputError::BaseVersionWithoutContent);
        }
        match (&self.content, replace.is_some() || append.is_some()) {
            (Some(_), true) => Err(WriteInputError::ContentWithEdits),
            (Some(content), false) => Ok(match self.base_version {
                Some(base_version) => WriteAction::Overwrite {
                    content: content.clone(),
                    base_version,
                },
                None => WriteAction::Create {
                    content: content.clone(),
                },
            }),
            (None, true) => Ok(WriteAction::Edit { replace, append }),
            (None, false) => Err(WriteInputError::NoAction),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(json: &str) -> WriteInput {
        serde_json::from_str(json).unwrap()
    }

    fn date(year: i32, month: time::Month, day: u8) -> Date {
        Date::from_calendar_date(year, month, day).unwrap()
    }

    #[test]
    fn valid_combinations_map_to_one_action() {
        let append = Append {
            text: "renewed".to_owned(),
            source: Some("call".to_owned()),
            date: Some(date(2026, time::Month::March, 2)),
        };
        let replacement = Replacement {
            old: "a".to_owned(),
            new: String::new(),
        };
        let cases = [
            (
                r#"{"path":"p","content":"x"}"#,
                WriteAction::Create {
                    content: "x".to_owned(),
                },
            ),
            (
                r#"{"path":"p","append_text":"renewed"}"#,
                WriteAction::Edit {
                    replace: None,
                    append: Some(Append {
                        text: "renewed".to_owned(),
                        source: None,
                        date: None,
                    }),
                },
            ),
            (
                r#"{"path":"p","replace_old":"a","replace_new":"","append_text":"renewed","append_source":"call","append_date":"2026-03-02"}"#,
                WriteAction::Edit {
                    replace: Some(replacement),
                    append: Some(append),
                },
            ),
            (
                r#"{"path":"p","content":"x","base_version":3}"#,
                WriteAction::Overwrite {
                    content: "x".to_owned(),
                    base_version: 3,
                },
            ),
            (
                r#"{"path":"p","content":"x","replace_old":null,"append_text":null,"base_version":null}"#,
                WriteAction::Create {
                    content: "x".to_owned(),
                },
            ),
        ];
        for (json, expected) in cases {
            assert_eq!(input(json).action(), Ok(expected), "{json}");
        }
    }

    #[test]
    fn invalid_combinations_are_rejected_by_cause() {
        let cases = [
            (r#"{"path":"p"}"#, WriteInputError::NoAction),
            (
                r#"{"path":"p","content":"x","append_text":"t"}"#,
                WriteInputError::ContentWithEdits,
            ),
            (
                r#"{"path":"p","content":"x","replace_old":"a","replace_new":"b"}"#,
                WriteInputError::ContentWithEdits,
            ),
            (
                r#"{"path":"p","replace_old":"a"}"#,
                WriteInputError::IncompleteReplace,
            ),
            (
                r#"{"path":"p","replace_new":"b","append_text":"t"}"#,
                WriteInputError::IncompleteReplace,
            ),
            (
                r#"{"path":"p","append_source":"call"}"#,
                WriteInputError::AppendWithoutText,
            ),
            (
                r#"{"path":"p","append_date":"2026-01-01"}"#,
                WriteInputError::AppendWithoutText,
            ),
            (
                r#"{"path":"p","append_text":"t","base_version":1}"#,
                WriteInputError::BaseVersionWithoutContent,
            ),
            (
                r#"{"path":"p","base_version":1}"#,
                WriteInputError::BaseVersionWithoutContent,
            ),
            (
                r#"{"path":"p","append_text":"t","append_date":"2026-02-30"}"#,
                WriteInputError::InvalidDate,
            ),
            (
                r#"{"path":"p","append_text":"t","append_date":"yesterday"}"#,
                WriteInputError::InvalidDate,
            ),
        ];
        for (json, expected) in cases {
            assert_eq!(input(json).action(), Err(expected), "{json}");
        }
    }
}
