use memory_ops::{FieldValue, HitKind, Page, SearchHit};

use crate::pages::read::format_time;
use crate::search::queries::matches::MatchRow;

const MAX_LINES_PER_PAGE: usize = 5;

pub(super) fn hits_for(row: &MatchRow, text: Option<&str>) -> Vec<SearchHit> {
    let at = format_time(row.created_at);
    let lines = text.map_or_else(Vec::new, |needle| matching_lines(&row.content, needle));
    if lines.is_empty() {
        return vec![SearchHit {
            kind: HitKind::Page,
            path: row.path.clone(),
            page: None,
            line_number: None,
            line: fields_line(&row.content),
            author: row.author_name.clone(),
            at,
        }];
    }
    lines
        .into_iter()
        .map(|(number, line)| SearchHit {
            kind: HitKind::Page,
            path: row.path.clone(),
            page: None,
            line_number: Some(number),
            line: Some(line),
            author: row.author_name.clone(),
            at: at.clone(),
        })
        .collect()
}

fn fields_line(content: &str) -> Option<String> {
    let page = Page::parse(content).ok()?;
    let line = page
        .fields()
        .iter()
        .map(|field| {
            let values = field
                .values()
                .iter()
                .map(FieldValue::text)
                .collect::<Vec<_>>()
                .join(", ");
            format!("{}: {values}", field.key())
        })
        .collect::<Vec<_>>()
        .join("; ");
    (!line.is_empty()).then_some(line)
}

pub(super) fn matching_lines(content: &str, needle: &str) -> Vec<(u32, String)> {
    let mut found: Vec<(u32, String)> = Vec::new();
    for (offset, _) in content.match_indices(needle) {
        let before = content.get(..offset).unwrap_or_default();
        let number = u32::try_from(before.matches('\n').count() + 1).unwrap_or(u32::MAX);
        if found.last().is_some_and(|(last, _)| *last == number) {
            continue;
        }
        let line_start = before.rfind('\n').map_or(0, |index| index + 1);
        let line = content
            .get(line_start..)
            .and_then(|rest| rest.lines().next())
            .unwrap_or_default();
        found.push((number, line.to_owned()));
        if found.len() == MAX_LINES_PER_PAGE {
            break;
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lines_are_numbered_from_one_and_listed_once() {
        let content = "alpha beta\nbeta again beta\n\n한글 검색\nbeta";
        let cases = [
            (
                "beta",
                vec![(1, "alpha beta"), (2, "beta again beta"), (5, "beta")],
            ),
            ("한글", vec![(4, "한글 검색")]),
            ("검색", vec![(4, "한글 검색")]),
            ("missing", vec![]),
            ("a\nb", vec![(1, "alpha beta")]),
            ("x\ny", vec![]),
        ];
        for (needle, expected) in cases {
            let expected: Vec<(u32, String)> = expected
                .into_iter()
                .map(|(number, line)| (number, line.to_owned()))
                .collect();
            assert_eq!(matching_lines(content, needle), expected, "{needle}");
        }
    }

    #[test]
    fn long_pages_report_a_bounded_number_of_lines() {
        let content = "x\n".repeat(50);
        assert_eq!(matching_lines(&content, "x").len(), MAX_LINES_PER_PAGE);
    }
}
