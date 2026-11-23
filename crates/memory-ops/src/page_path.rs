use std::fmt;

use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PagePath(String);

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum PagePathError {
    #[error("path is empty")]
    Empty,
    #[error("path has an empty segment")]
    EmptySegment,
    #[error("path has a `..` segment")]
    ParentSegment,
    #[error("path has a control character")]
    ControlCharacter,
}

impl PagePath {
    pub fn parse(input: &str) -> Result<Self, PagePathError> {
        let normalized: String = input.nfc().collect();
        if normalized.is_empty() {
            return Err(PagePathError::Empty);
        }
        if normalized.chars().any(char::is_control) {
            return Err(PagePathError::ControlCharacter);
        }
        for segment in normalized.split('/') {
            match segment {
                "" => return Err(PagePathError::EmptySegment),
                ".." => return Err(PagePathError::ParentSegment),
                _ => {}
            }
        }
        Ok(Self(normalized))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PagePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decomposed_hangul_is_stored_composed() {
        let path = PagePath::parse("clients/\u{1112}\u{1161}\u{11AB}").unwrap();
        assert_eq!(path.as_str(), "clients/\u{D55C}");
    }

    #[test]
    fn invalid_paths_are_rejected_by_cause() {
        let cases = [
            ("", PagePathError::Empty),
            ("a//b", PagePathError::EmptySegment),
            ("/a", PagePathError::EmptySegment),
            ("a/", PagePathError::EmptySegment),
            ("a/../b", PagePathError::ParentSegment),
            ("a/b\nc", PagePathError::ControlCharacter),
            ("a\0b", PagePathError::ControlCharacter),
        ];
        for (input, expected) in cases {
            assert_eq!(PagePath::parse(input), Err(expected), "{input:?}");
        }
    }

    #[test]
    fn dotted_names_are_not_parent_segments() {
        assert!(PagePath::parse("notes/..draft").is_ok());
    }
}
