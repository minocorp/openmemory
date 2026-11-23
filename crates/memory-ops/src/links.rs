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

fn links(text: &str) -> Vec<Link<'_>> {
    let mut found = Vec::new();
    let mut position = 0;
    while let Some(offset) = text.get(position..).and_then(|rest| rest.find("[[")) {
        let start = position + offset;
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
    fn rewriting_without_a_match_returns_none() {
        assert_eq!(rewrite_links("[[a]]", &path("b"), &path("c")), None);
    }

    #[test]
    fn decomposed_targets_match_composed_paths() {
        let decomposed = "[[\u{1112}\u{1161}\u{11AB}]]";
        assert_eq!(link_targets(decomposed), vec![path("\u{D55C}")]);
    }
}
