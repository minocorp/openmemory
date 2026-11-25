use axum::http::header::{IF_MATCH, IF_NONE_MATCH};
use axum::http::{HeaderMap, HeaderValue};

use crate::api::error::ApiError;
use crate::pages::ids::VersionNumber;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum IfMatch {
    Any,
    Versions(Vec<i32>),
}

pub(super) fn etag(version: VersionNumber) -> HeaderValue {
    HeaderValue::from_str(&format!("\"{}\"", version.0))
        .unwrap_or_else(|_| HeaderValue::from_static("\"0\""))
}

pub(super) fn if_match(headers: &HeaderMap) -> Result<IfMatch, ApiError> {
    let mut tokens = headers
        .get_all(IF_MATCH)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .peekable();
    if tokens.peek().is_none() {
        return Err(ApiError::PreconditionRequired);
    }
    let mut versions = Vec::new();
    for token in tokens {
        if token == "*" {
            return Ok(IfMatch::Any);
        }
        if let Some(version) = strong_version(token) {
            versions.push(version);
        }
    }
    Ok(IfMatch::Versions(versions))
}

pub(super) fn requires_absent(headers: &HeaderMap) -> Result<(), ApiError> {
    let value = headers
        .get(IF_NONE_MATCH)
        .and_then(|value| value.to_str().ok())
        .ok_or(ApiError::PreconditionRequired)?;
    if value.trim() == "*" {
        Ok(())
    } else {
        Err(ApiError::InvalidInput("if-none-match"))
    }
}

pub(super) fn matches_none_match(headers: &HeaderMap, current: &HeaderValue) -> bool {
    let Some(current) = current.to_str().ok().map(weak_stripped) else {
        return false;
    };
    headers
        .get_all(IF_NONE_MATCH)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .map(str::trim)
        .any(|token| token == "*" || weak_stripped(token) == current)
}

impl IfMatch {
    pub(super) fn expected(&self, current: VersionNumber) -> VersionNumber {
        match self {
            Self::Any => current,
            Self::Versions(versions) if versions.contains(&current.0) => current,
            Self::Versions(versions) => VersionNumber(versions.first().copied().unwrap_or(0)),
        }
    }
}

fn strong_version(token: &str) -> Option<i32> {
    token
        .strip_prefix('"')?
        .strip_suffix('"')?
        .parse::<i32>()
        .ok()
}

fn weak_stripped(token: &str) -> &str {
    token.strip_prefix("W/").unwrap_or(token)
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    fn headers(name: &'static str, value: &'static str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(name, HeaderValue::from_static(value));
        headers
    }

    #[test]
    fn a_missing_if_match_is_required() {
        assert!(matches!(
            if_match(&HeaderMap::new()),
            Err(ApiError::PreconditionRequired)
        ));
    }

    #[test]
    fn if_match_accepts_strong_tags_lists_and_the_star() {
        let cases = [
            ("\"3\"", IfMatch::Versions(vec![3])),
            ("\"3\", \"5\"", IfMatch::Versions(vec![3, 5])),
            ("W/\"3\"", IfMatch::Versions(vec![])),
            ("garbage", IfMatch::Versions(vec![])),
            ("*", IfMatch::Any),
        ];
        for (value, expected) in cases {
            assert_eq!(
                if_match(&headers("if-match", value)).unwrap(),
                expected,
                "{value}"
            );
        }
    }

    #[test]
    fn the_expected_version_is_the_current_one_only_when_listed() {
        let current = VersionNumber(5);
        assert_eq!(IfMatch::Any.expected(current), current);
        assert_eq!(IfMatch::Versions(vec![3, 5]).expected(current), current);
        assert_eq!(
            IfMatch::Versions(vec![3]).expected(current),
            VersionNumber(3)
        );
        assert_eq!(
            IfMatch::Versions(vec![]).expected(current),
            VersionNumber(0)
        );
    }

    #[test]
    fn if_none_match_uses_weak_comparison() {
        let current = etag(VersionNumber(4));
        assert!(matches_none_match(
            &headers("if-none-match", "W/\"4\""),
            &current
        ));
        assert!(matches_none_match(&headers("if-none-match", "*"), &current));
        assert!(!matches_none_match(
            &headers("if-none-match", "\"3\""),
            &current
        ));
    }

    #[test]
    fn creating_needs_if_none_match_star() {
        assert!(requires_absent(&headers("if-none-match", "*")).is_ok());
        assert!(matches!(
            requires_absent(&HeaderMap::new()),
            Err(ApiError::PreconditionRequired)
        ));
        assert!(matches!(
            requires_absent(&headers("if-none-match", "\"1\"")),
            Err(ApiError::InvalidInput(_))
        ));
    }
}
