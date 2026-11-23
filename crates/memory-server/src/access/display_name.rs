use std::fmt;

use serde::{Deserialize, Deserializer};
use unicode_normalization::UnicodeNormalization;

const MAX_CHARS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, utoipa::ToSchema)]
#[schema(value_type = String, min_length = 1, max_length = 64)]
pub(crate) struct DisplayName(String);

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum DisplayNameError {
    #[error("display name is empty")]
    Empty,
    #[error("display name is too long")]
    TooLong,
    #[error("display name has a control character")]
    Forbidden,
}

impl DisplayName {
    pub(crate) fn parse(input: &str) -> Result<Self, DisplayNameError> {
        let normalized: String = input.trim().nfc().collect();
        if normalized.is_empty() {
            return Err(DisplayNameError::Empty);
        }
        if normalized.chars().count() > MAX_CHARS {
            return Err(DisplayNameError::TooLong);
        }
        if normalized.chars().any(char::is_control) {
            return Err(DisplayNameError::Forbidden);
        }
        Ok(Self(normalized))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DisplayName {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for DisplayName {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_names_may_hold_any_text_but_not_control_characters() {
        assert_eq!(
            DisplayName::parse("  Mino Kim ").unwrap().as_str(),
            "Mino Kim"
        );
        assert_eq!(
            DisplayName::parse("민호 (디자인)").unwrap().as_str(),
            "민호 (디자인)"
        );
        assert_eq!(DisplayName::parse("a/b").unwrap().as_str(), "a/b");
        assert_eq!(DisplayName::parse(""), Err(DisplayNameError::Empty));
        assert_eq!(
            DisplayName::parse(&"a".repeat(65)),
            Err(DisplayNameError::TooLong)
        );
        assert_eq!(DisplayName::parse("a\nb"), Err(DisplayNameError::Forbidden));
    }
}
