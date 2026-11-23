use std::fmt;

use time::Date;

use crate::PagePath;
use crate::iso_date::{format_iso_date, parse_iso_date};
use crate::page::error::PageErrorKind;

const FIELD_SEPARATOR: &str = " | ";
const TEXT_SEPARATOR: &str = " — ";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineEntry {
    raw: String,
    date: Date,
    source: Source,
    text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Source {
    Text(String),
    File {
        path: PagePath,
        page: Option<u32>,
        quote: Option<String>,
    },
}

impl TimelineEntry {
    pub fn parse(line: &str) -> Result<Self, PageErrorKind> {
        let date = line
            .get(..10)
            .and_then(parse_iso_date)
            .ok_or(PageErrorKind::TimelineDate)?;
        let rest = line
            .get(10..)
            .and_then(|rest| rest.strip_prefix(FIELD_SEPARATOR))
            .ok_or(PageErrorKind::TimelineSeparator)?;
        let split = text_separator(rest).ok_or(PageErrorKind::TimelineSource)?;
        let source = rest.get(..split).unwrap_or_default();
        let text = rest.get(split + TEXT_SEPARATOR.len()..).unwrap_or_default();
        if source.trim().is_empty() {
            return Err(PageErrorKind::TimelineSource);
        }
        if text.trim().is_empty() {
            return Err(PageErrorKind::TimelineText);
        }
        Ok(Self {
            raw: line.to_owned(),
            date,
            source: Source::parse(source),
            text: text.to_owned(),
        })
    }

    pub fn compose(date: Date, source: &str, text: &str) -> Result<Self, PageErrorKind> {
        let has_break = |value: &str| value.contains(['\n', '\r']);
        if has_break(source) {
            return Err(PageErrorKind::TimelineSource);
        }
        if has_break(text) {
            return Err(PageErrorKind::TimelineText);
        }
        let entry = Self::parse(&format!(
            "{}{FIELD_SEPARATOR}{source}{TEXT_SEPARATOR}{text}",
            format_iso_date(date)
        ))?;
        if entry.source.to_string() != source {
            return Err(PageErrorKind::TimelineSource);
        }
        Ok(entry)
    }

    pub fn raw(&self) -> &str {
        &self.raw
    }

    pub fn date(&self) -> Date {
        self.date
    }

    pub fn source(&self) -> &Source {
        &self.source
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

impl Source {
    fn parse(text: &str) -> Self {
        let (reference, quote) = split_quote(text);
        let (path, page) = split_page(reference);
        match PagePath::parse(path) {
            Ok(path)
                if path.as_str().contains('/') && !path.as_str().contains(char::is_whitespace) =>
            {
                Self::File { path, page, quote }
            }
            Ok(_) | Err(_) => Self::Text(text.to_owned()),
        }
    }
}

impl fmt::Display for Source {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(text) => formatter.write_str(text),
            Self::File { path, page, quote } => {
                write!(formatter, "{path}")?;
                if let Some(page) = page {
                    write!(formatter, "#p{page}")?;
                }
                if let Some(quote) = quote {
                    write!(formatter, " \"{quote}\"")?;
                }
                Ok(())
            }
        }
    }
}

fn text_separator(rest: &str) -> Option<usize> {
    let mut in_quote = false;
    for (index, character) in rest.char_indices() {
        if character == '"' {
            in_quote = !in_quote;
        } else if !in_quote && rest.get(index..)?.starts_with(TEXT_SEPARATOR) {
            return Some(index);
        }
    }
    None
}

fn split_quote(source: &str) -> (&str, Option<String>) {
    let Some(body) = source.strip_suffix('"') else {
        return (source, None);
    };
    match body.split_once(" \"") {
        Some((reference, quote)) if !quote.contains('"') => (reference, Some(quote.to_owned())),
        Some(_) | None => (source, None),
    }
}

fn split_page(reference: &str) -> (&str, Option<u32>) {
    let Some((path, marker)) = reference.rsplit_once("#p") else {
        return (reference, None);
    };
    match marker.parse::<u32>() {
        Ok(page) => (path, Some(page)),
        Err(_) => (reference, None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, page: Option<u32>, quote: Option<&str>) -> Source {
        Source::File {
            path: PagePath::parse(path).unwrap(),
            page,
            quote: quote.map(str::to_owned),
        }
    }

    #[test]
    fn lines_parse_into_date_source_and_text() {
        let cases = [
            (
                "2026-11-20 | call — agreed to lower the fee",
                Source::Text("call".to_owned()),
                "agreed to lower the fee",
            ),
            (
                "2026-03-02 | contracts/acme.pdf#p2 \"월 금 오십만원정\" — contract signed, 500,000",
                file("contracts/acme.pdf", Some(2), Some("월 금 오십만원정")),
                "contract signed, 500,000",
            ),
            (
                "2026-03-02 | contracts/acme.pdf — signed",
                file("contracts/acme.pdf", None, None),
                "signed",
            ),
            (
                "2026-03-02 | contracts/acme.pdf \"a — b\" — quoted dash",
                file("contracts/acme.pdf", None, Some("a — b")),
                "quoted dash",
            ),
            (
                "2026-03-02 | meeting notes — free text source",
                Source::Text("meeting notes".to_owned()),
                "free text source",
            ),
            (
                "2026-03-02 | call — text — with another dash",
                Source::Text("call".to_owned()),
                "text — with another dash",
            ),
        ];
        for (line, source, text) in cases {
            let entry = TimelineEntry::parse(line).unwrap();
            assert_eq!(entry.source(), &source, "{line}");
            assert_eq!(entry.text(), text, "{line}");
            assert_eq!(entry.raw(), line);
        }
    }

    #[test]
    fn invalid_lines_are_rejected_by_cause() {
        let cases = [
            ("call — text", PageErrorKind::TimelineDate),
            ("2026-13-01 | call — text", PageErrorKind::TimelineDate),
            ("2026-11-20 call — text", PageErrorKind::TimelineSeparator),
            ("2026-11-20 | call text", PageErrorKind::TimelineSource),
            ("2026-11-20 |  — text", PageErrorKind::TimelineSource),
            ("2026-11-20 | call — ", PageErrorKind::TimelineText),
        ];
        for (line, expected) in cases {
            assert_eq!(TimelineEntry::parse(line), Err(expected), "{line}");
        }
    }

    #[test]
    fn composed_lines_use_the_canonical_form() {
        let date = Date::from_calendar_date(2026, time::Month::November, 3).unwrap();
        let entry = TimelineEntry::compose(date, "call", "renewed").unwrap();
        assert_eq!(entry.raw(), "2026-11-03 | call — renewed");
        assert_eq!(entry.date(), date);
    }

    #[test]
    fn sources_display_as_written() {
        let source = file("contracts/acme.pdf", Some(2), Some("월 금"));
        assert_eq!(source.to_string(), "contracts/acme.pdf#p2 \"월 금\"");
    }
}
