use std::fmt;

use serde::{Deserialize, Deserializer};
use unicode_normalization::UnicodeNormalization;

const MAX_CHARS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, utoipa::ToSchema)]
pub(crate) struct Name(String);

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum NameError {
    #[error("name is empty")]
    Empty,
    #[error("name is too long")]
    TooLong,
    #[error("name has a slash or control character")]
    Forbidden,
}

impl Name {
    pub(crate) fn parse(input: &str) -> Result<Self, NameError> {
        let normalized: String = input.trim().nfc().collect();
        if normalized.is_empty() {
            return Err(NameError::Empty);
        }
        if normalized.chars().count() > MAX_CHARS {
            return Err(NameError::TooLong);
        }
        if normalized.chars().any(|c| c == '/' || c.is_control()) {
            return Err(NameError::Forbidden);
        }
        Ok(Self(normalized))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Name {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Name {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_trimmed_and_checked_by_cause() {
        assert_eq!(
            Name::parse("  claude-code ").unwrap().as_str(),
            "claude-code"
        );
        assert_eq!(Name::parse(" "), Err(NameError::Empty));
        assert_eq!(Name::parse(&"a".repeat(65)), Err(NameError::TooLong));
        assert_eq!(Name::parse("a/b"), Err(NameError::Forbidden));
        assert_eq!(Name::parse("a\nb"), Err(NameError::Forbidden));
    }
}
