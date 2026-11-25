use std::fmt;

use serde::{Deserialize, Deserializer};

const MAX_CHARS: usize = 32;

#[derive(Debug, Clone, PartialEq, Eq, utoipa::ToSchema)]
#[schema(value_type = String, pattern = "^[a-z0-9_.-]{1,32}$")]
pub(crate) struct Username(String);

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum UsernameError {
    #[error("username is empty")]
    Empty,
    #[error("username is too long")]
    TooLong,
    #[error("username has a character other than letters, digits, dash, underscore or dot")]
    Forbidden,
}

impl Username {
    pub(crate) fn parse(input: &str) -> Result<Self, UsernameError> {
        let lowered = input.trim().to_lowercase();
        if lowered.is_empty() {
            return Err(UsernameError::Empty);
        }
        if lowered.chars().count() > MAX_CHARS {
            return Err(UsernameError::TooLong);
        }
        if !lowered
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_' | '.'))
        {
            return Err(UsernameError::Forbidden);
        }
        Ok(Self(lowered))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Username {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for Username {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::parse(&text).map_err(serde::de::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usernames_are_lowercased_and_checked_by_cause() {
        assert_eq!(
            Username::parse(" Mino.Kim_1 ").unwrap().as_str(),
            "mino.kim_1"
        );
        assert_eq!(Username::parse(" "), Err(UsernameError::Empty));
        assert_eq!(
            Username::parse(&"a".repeat(33)),
            Err(UsernameError::TooLong)
        );
        assert_eq!(
            Username::parse("a".repeat(32).as_str())
                .unwrap()
                .as_str()
                .len(),
            32
        );
        assert_eq!(Username::parse("a b"), Err(UsernameError::Forbidden));
        assert_eq!(Username::parse("a/b"), Err(UsernameError::Forbidden));
        assert_eq!(Username::parse("민호"), Err(UsernameError::Forbidden));
    }
}
