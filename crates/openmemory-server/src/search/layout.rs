use std::collections::HashMap;

use crate::search::candidate::Candidate;
use crate::search::position::Position;

const PREVIEW: usize = 3;

#[derive(Debug)]
pub(crate) struct Layout<'a> {
    pub(crate) hits: Vec<&'a Candidate>,
    pub(crate) folders: Vec<Group<'a>>,
    pub(crate) next: Option<Position>,
}

#[derive(Debug)]
pub(crate) struct Group<'a> {
    pub(crate) path: String,
    pub(crate) matches: usize,
    pub(crate) preview: Vec<&'a Candidate>,
}

pub(crate) fn lay_out<'a>(
    newest_first: &'a [Candidate],
    folder: &str,
    after: Option<Position>,
    limit: usize,
) -> Layout<'a> {
    let scope = common_folder(newest_first, folder);
    let mut direct = Vec::new();
    let mut folders: Vec<Group<'a>> = Vec::new();
    let mut group_of: HashMap<String, usize> = HashMap::new();
    for candidate in newest_first {
        let Some(path) = child_folder(&candidate.path, &scope) else {
            direct.push(candidate);
            continue;
        };
        let index = *group_of.entry(path.clone()).or_insert_with(|| {
            folders.push(Group {
                path,
                matches: 0,
                preview: Vec::new(),
            });
            folders.len() - 1
        });
        if let Some(group) = folders.get_mut(index) {
            group.matches += 1;
            if group.preview.len() < PREVIEW {
                group.preview.push(candidate);
            }
        }
    }
    let mut hits: Vec<&Candidate> = direct
        .into_iter()
        .filter(|candidate| after.is_none_or(|after| candidate.position() < after))
        .take(limit + 1)
        .collect();
    let next = if hits.len() > limit {
        hits.truncate(limit);
        hits.last().map(|candidate| candidate.position())
    } else {
        None
    };
    if after.is_some() {
        folders.clear();
    }
    folders.truncate(limit);
    Layout {
        hits,
        folders,
        next,
    }
}

fn common_folder(candidates: &[Candidate], requested: &str) -> String {
    let mut common: Option<Vec<&str>> = None;
    for candidate in candidates {
        let segments: Vec<&str> = candidate
            .path
            .rsplit_once('/')
            .map_or_else(Vec::new, |(parent, _)| parent.split('/').collect());
        common = Some(match common {
            None => segments,
            Some(previous) => previous
                .into_iter()
                .zip(segments)
                .take_while(|(left, right)| left == right)
                .map(|(segment, _)| segment)
                .collect(),
        });
    }
    let common = common.unwrap_or_default().join("/");
    if common.len() >= requested.len() {
        common
    } else {
        requested.to_owned()
    }
}

fn child_folder(path: &str, scope: &str) -> Option<String> {
    let rest = if scope.is_empty() {
        path
    } else {
        path.strip_prefix(scope)?.strip_prefix('/')?
    };
    let (child, _) = rest.split_once('/')?;
    Some(if scope.is_empty() {
        child.to_owned()
    } else {
        format!("{scope}/{child}")
    })
}

#[cfg(test)]
mod tests {
    use time::OffsetDateTime;

    use super::*;
    use crate::pages::ids::PageId;
    use crate::search::position::Document;

    fn newest_first(paths: &[&str]) -> Vec<Candidate> {
        let count = i64::try_from(paths.len()).unwrap();
        paths
            .iter()
            .zip(0..)
            .map(|(path, index)| Candidate {
                document: Document::Page(PageId(index)),
                path: (*path).to_owned(),
                author: "mino".to_owned(),
                at: OffsetDateTime::from_unix_timestamp(count - index).unwrap(),
            })
            .collect()
    }

    fn hit_paths<'a>(layout: &Layout<'a>) -> Vec<&'a str> {
        layout.hits.iter().map(|hit| hit.path.as_str()).collect()
    }

    fn folder_summary(layout: &Layout<'_>) -> Vec<(String, usize, Vec<String>)> {
        layout
            .folders
            .iter()
            .map(|group| {
                let preview = group.preview.iter().map(|hit| hit.path.clone()).collect();
                (group.path.clone(), group.matches, preview)
            })
            .collect()
    }

    #[test]
    fn a_large_folder_does_not_hide_the_small_ones() {
        let mut paths: Vec<String> = (0..50)
            .map(|index| format!("archive/posts/{index}"))
            .collect();
        paths.insert(20, "business/tax".to_owned());
        paths.push("start".to_owned());
        let paths: Vec<&str> = paths.iter().map(String::as_str).collect();
        let candidates = newest_first(&paths);
        let layout = lay_out(&candidates, "", None, 20);
        assert_eq!(hit_paths(&layout), ["start"]);
        assert_eq!(
            folder_summary(&layout),
            [
                (
                    "archive".to_owned(),
                    50,
                    vec![
                        "archive/posts/0".to_owned(),
                        "archive/posts/1".to_owned(),
                        "archive/posts/2".to_owned()
                    ]
                ),
                ("business".to_owned(), 1, vec!["business/tax".to_owned()]),
            ]
        );
        assert_eq!(layout.next, None);
    }

    #[test]
    fn matches_inside_one_folder_are_listed_from_that_folder() {
        let candidates = newest_first(&["a/b/one", "a/b/c/two", "a/b/three"]);
        let layout = lay_out(&candidates, "", None, 20);
        assert_eq!(hit_paths(&layout), ["a/b/one", "a/b/three"]);
        assert_eq!(
            folder_summary(&layout),
            [("a/b/c".to_owned(), 1, vec!["a/b/c/two".to_owned()])]
        );
    }

    #[test]
    fn a_requested_folder_that_is_also_a_page_lists_that_page() {
        let candidates = newest_first(&["clients/acme", "clients/acme/notes"]);
        let layout = lay_out(&candidates, "clients/acme", None, 20);
        assert_eq!(hit_paths(&layout), ["clients/acme", "clients/acme/notes"]);
        assert!(layout.folders.is_empty());
    }

    #[test]
    fn folders_match_whole_segments() {
        let candidates = newest_first(&["clients/acme", "clients2/other"]);
        let layout = lay_out(&candidates, "", None, 20);
        assert_eq!(layout.hits.len(), 0);
        assert_eq!(
            folder_summary(&layout)
                .into_iter()
                .map(|(path, ..)| path)
                .collect::<Vec<_>>(),
            ["clients", "clients2"]
        );
    }

    #[test]
    fn the_cursor_pages_through_direct_hits_and_folders_come_first() {
        let candidates = newest_first(&["x/0", "x/1", "x/2", "x/3", "x/4"]);
        let first = lay_out(&candidates, "", None, 2);
        assert_eq!(hit_paths(&first), ["x/0", "x/1"]);
        let second = lay_out(&candidates, "", first.next, 2);
        assert_eq!(hit_paths(&second), ["x/2", "x/3"]);
        let third = lay_out(&candidates, "", second.next, 2);
        assert_eq!(hit_paths(&third), ["x/4"]);
        assert_eq!(third.next, None);
    }

    #[test]
    fn folders_are_capped_by_the_limit_and_left_out_of_later_pages() {
        let candidates = newest_first(&["a/1", "b/1", "c/1", "top"]);
        let first = lay_out(&candidates, "", None, 2);
        assert_eq!(first.folders.len(), 2);
        let later = lay_out(
            &candidates,
            "",
            candidates.first().map(Candidate::position),
            2,
        );
        assert!(later.folders.is_empty());
        assert_eq!(hit_paths(&later), ["top"]);
    }
}
