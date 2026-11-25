use std::ops::Range;

use pulldown_cmark::{Event, Options, Parser, Tag};

use crate::PagePath;

struct Link<'a> {
    start: usize,
    end: usize,
    inner: &'a str,
}

pub fn link_targets(text: &str) -> Vec<PagePath> {
    let mut targets: Vec<PagePath> = Vec::new();
    for link in links(text) {
        if let Ok(path) = PagePath::parse(link.inner)
            && !targets.contains(&path)
        {
            targets.push(path);
        }
    }
    targets
}

pub fn rewrite_links(text: &str, from: &PagePath, to: &PagePath) -> Option<String> {
    let mut rewritten = String::with_capacity(text.len());
    let mut copied = 0;
    for link in links(text) {
        if PagePath::parse(link.inner).as_ref() == Ok(from) {
            rewritten.push_str(text.get(copied..link.start)?);
            rewritten.push_str("[[");
            rewritten.push_str(to.as_str());
            rewritten.push_str("]]");
            copied = link.end;
        }
    }
    if copied == 0 {
        return None;
    }
    rewritten.push_str(text.get(copied..)?);
    Some(rewritten)
}

fn code_spans(text: &str) -> Vec<Range<usize>> {
    let options = Options::ENABLE_TABLES
        | Options::ENABLE_STRIKETHROUGH
        | Options::ENABLE_TASKLISTS
        | Options::ENABLE_YAML_STYLE_METADATA_BLOCKS;
    Parser::new_ext(text, options)
        .into_offset_iter()
        .filter(|(event, _)| matches!(event, Event::Code(_) | Event::Start(Tag::CodeBlock(_))))
        .map(|(_, range)| range)
        .collect()
}

fn links(text: &str) -> Vec<Link<'_>> {
    let code = code_spans(text);
    let mut found = Vec::new();
    let mut position = 0;
    while let Some(offset) = text.get(position..).and_then(|rest| rest.find("[[")) {
        let start = position + offset;
        if let Some(span) = code.iter().find(|span| span.contains(&start)) {
            position = span.end.max(start + 1);
            continue;
        }
        if text
            .get(..start)
            .is_some_and(|before| before.ends_with('\\'))
        {
            position = start + 1;
            continue;
        }
        let inner_start = start + 2;
        let Some(length) = text.get(inner_start..).and_then(|rest| rest.find("]]")) else {
            break;
        };
        let inner = text
            .get(inner_start..inner_start + length)
            .unwrap_or_default();
        if inner.contains(['\n', '[']) {
            position = inner_start;
            continue;
        }
        let end = inner_start + length + 2;
        found.push(Link { start, end, inner });
        position = end;
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(value: &str) -> PagePath {
        PagePath::parse(value).unwrap()
    }

    #[test]
    fn targets_are_unique_valid_paths_in_order() {
        let text =
            "see [[clients/acme]] and [[tax]], again [[clients/acme]], bad [[a//b]], open [[x";
        assert_eq!(link_targets(text), vec![path("clients/acme"), path("tax")]);
    }

    #[test]
    fn links_spanning_lines_or_nesting_are_ignored() {
        assert_eq!(link_targets("[[a\nb]] [[[c]]").len(), 0);
    }

    #[test]
    fn rewriting_replaces_only_exact_targets() {
        let text = "[[old]] [[older]] [[old/sub]] and [[old]] again 한글";
        let rewritten = rewrite_links(text, &path("old"), &path("new/place")).unwrap();
        assert_eq!(
            rewritten,
            "[[new/place]] [[older]] [[old/sub]] and [[new/place]] again 한글"
        );
    }

    #[test]
    fn links_inside_code_are_ignored() {
        let text = "`[[a]]` and [[b]]\n\n```\n[[c]]\n```\n\n    [[d]]\n\n[[e]]";
        assert_eq!(link_targets(text), vec![path("b"), path("e")]);
    }

    #[test]
    fn indented_frontmatter_values_are_not_code() {
        let text = "---\nlinks:\n    - \"[[a]]\"\n---\n\n[[b]]";
        assert_eq!(link_targets(text), vec![path("a"), path("b")]);
    }

    #[test]
    fn escaped_links_are_ignored() {
        assert_eq!(link_targets("\\[[a]] [[b]]"), vec![path("b")]);
    }

    #[test]
    fn rewriting_leaves_code_alone() {
        let rewritten = rewrite_links("`[[old]]` [[old]]", &path("old"), &path("new")).unwrap();
        assert_eq!(rewritten, "`[[old]]` [[new]]");
    }

    #[test]
    fn rewriting_without_a_match_returns_none() {
        assert_eq!(rewrite_links("[[a]]", &path("b"), &path("c")), None);
    }

    #[test]
    fn decomposed_targets_match_composed_paths() {
        let decomposed = "[[\u{1112}\u{1161}\u{11AB}]]";
        assert_eq!(link_targets(decomposed), vec![path("\u{D55C}")]);
    }
}
