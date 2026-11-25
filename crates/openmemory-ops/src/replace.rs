#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum ReplaceError {
    #[error("text to replace is empty")]
    EmptyOld,
    #[error("text to replace was not found")]
    NotFound,
    #[error("text to replace matches more than once")]
    Ambiguous { count: usize },
}

pub fn replace_unique(text: &str, old: &str, new: &str) -> Result<String, ReplaceError> {
    if old.is_empty() {
        return Err(ReplaceError::EmptyOld);
    }
    let mut matches = text
        .char_indices()
        .filter(|(index, _)| text.get(*index..).is_some_and(|rest| rest.starts_with(old)))
        .map(|(index, _)| index);
    let first = matches.next().ok_or(ReplaceError::NotFound)?;
    let others = matches.count();
    if others > 0 {
        return Err(ReplaceError::Ambiguous { count: others + 1 });
    }
    let before = text.get(..first).unwrap_or_default();
    let after = text.get(first + old.len()..).unwrap_or_default();
    Ok(format!("{before}{new}{after}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exactly_one_match_is_replaced() {
        assert_eq!(
            replace_unique("fee 500,000 KRW 월", "500,000", "450,000").as_deref(),
            Ok("fee 450,000 KRW 월")
        );
    }

    #[test]
    fn matches_are_rejected_by_count() {
        let cases = [
            ("abc", "x", ReplaceError::NotFound),
            ("abab", "ab", ReplaceError::Ambiguous { count: 2 }),
            ("aaa", "aa", ReplaceError::Ambiguous { count: 2 }),
            ("abc", "", ReplaceError::EmptyOld),
        ];
        for (text, old, expected) in cases {
            assert_eq!(
                replace_unique(text, old, "z"),
                Err(expected),
                "{text}/{old}"
            );
        }
    }
}
