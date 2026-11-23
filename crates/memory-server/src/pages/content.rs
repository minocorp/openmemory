use memory_ops::{Page, scan_credentials};

use crate::pages::error::PagesError;

const MAX_PAGE_BYTES: usize = 1_048_576;

#[derive(Debug)]
pub(super) struct Prepared {
    pub(super) text: String,
    pub(super) page: Page,
}

pub(super) fn prepare(text: &str) -> Result<Prepared, PagesError> {
    let text = text.replace('\0', "");
    if text.len() > MAX_PAGE_BYTES {
        return Err(PagesError::TooLarge);
    }
    let page = Page::parse(&text)?;
    Ok(Prepared { text, page })
}

pub(super) fn reject_credentials(text: &str) -> Result<(), PagesError> {
    match scan_credentials(text) {
        Some(pattern) => Err(PagesError::Credential(pattern)),
        None => Ok(()),
    }
}

pub(super) fn require_append_only(previous: &str, next: &Page) -> Result<(), PagesError> {
    let previous = Page::parse(previous).unwrap_or_else(|_| Page::default());
    if next.extends_timeline_of(&previous) {
        Ok(())
    } else {
        Err(PagesError::TimelineChanged)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nul_bytes_are_stripped_and_size_is_capped() {
        assert_eq!(prepare("a\0b").unwrap().text, "ab");
        let big = "a".repeat(MAX_PAGE_BYTES + 1);
        assert!(matches!(prepare(&big), Err(PagesError::TooLarge)));
        assert!(prepare(&"a".repeat(MAX_PAGE_BYTES)).is_ok());
    }

    #[test]
    fn grammar_errors_carry_the_line() {
        match prepare("## Timeline\nnot a line") {
            Err(PagesError::Grammar(error)) => assert_eq!(error.line(), 2),
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn credentials_are_rejected_with_the_pattern() {
        assert!(matches!(
            reject_credentials("card 4111 1111 1111 1111"),
            Err(PagesError::Credential(_))
        ));
        assert!(reject_credentials("fee 450,000").is_ok());
    }
}
