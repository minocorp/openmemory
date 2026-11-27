use std::fmt::Write;

use openmemory_api::{
    Changes, EntryKind, FolderEntry, HitKind, Operation, SearchHit, SearchResults, VersionList,
    VersionView,
};

use crate::output::Output;

pub(crate) fn entries(listing: &[FolderEntry]) -> String {
    let mut text = String::new();
    for entry in listing {
        let suffix = if entry.kind == EntryKind::Folder {
            "/"
        } else {
            ""
        };
        let _ = writeln!(text, "{}{suffix}", entry.name);
    }
    text
}

pub(crate) fn search(results: &SearchResults, output: Output) -> String {
    let mut text = String::new();
    for hit in &results.hits {
        text.push_str(&hit_line(hit, output, ""));
    }
    for folder in &results.folders {
        let count = if folder.matches == 1 {
            "1 match".to_owned()
        } else {
            format!("{} matches", folder.matches)
        };
        let _ = writeln!(
            text,
            "{}/  {}",
            output.bold(&folder.path),
            output.dim(&count)
        );
        for hit in &folder.hits {
            text.push_str(&hit_line(hit, output, "  "));
        }
    }
    text
}

fn hit_line(hit: &SearchHit, output: Output, indent: &str) -> String {
    let place = match (hit.kind, hit.page, hit.line_number) {
        (HitKind::File, Some(page), _) => format!(" (page {page})"),
        (HitKind::Page, _, Some(line)) => format!(":{line}"),
        _ => String::new(),
    };
    let line = hit.line.as_deref().unwrap_or_default();
    format!("{indent}{}{place}  {line}\n", output.bold(&hit.path))
}

pub(crate) fn versions(list: &VersionList, output: Output) -> String {
    let mut text = String::new();
    for version in &list.versions {
        text.push_str(&version_block(version, output));
    }
    text
}

fn version_block(version: &VersionView, output: Output) -> String {
    let mut text = String::new();
    let header = format!(
        "v{}  {}  {}  {}",
        version.version,
        operation(version.op),
        version.at,
        version.author
    );
    let _ = writeln!(text, "{}", output.bold(&header));
    text.push_str(&diff(&version.diff, output));
    text
}

pub(crate) fn changes(changes: &Changes, output: Output) -> String {
    let mut text = String::new();
    for group in &changes.groups {
        let task = group.task.as_deref().unwrap_or("no task");
        let header = format!("{}  {}  {task}", group.at, group.author);
        let _ = writeln!(text, "{}", output.bold(&header));
        for version in &group.versions {
            let _ = writeln!(
                text,
                "  {} {} v{}",
                operation(version.op),
                version.path,
                version.version
            );
        }
    }
    text
}

fn diff(text: &str, output: Output) -> String {
    let mut painted = String::new();
    for line in text.lines() {
        let styled = if line.starts_with('+') && !line.starts_with("+++") {
            output.green(line)
        } else if line.starts_with('-') && !line.starts_with("---") {
            output.red(line)
        } else {
            output.dim(line)
        };
        let _ = writeln!(painted, "  {styled}");
    }
    painted
}

fn operation(op: Operation) -> &'static str {
    match op {
        Operation::Create => "create",
        Operation::Edit => "edit",
        Operation::Move => "move",
        Operation::Relink => "relink",
        Operation::Delete => "delete",
        Operation::Revert => "revert",
    }
}

#[cfg(test)]
mod tests {
    use openmemory_api::{ChangeGroup, ChangedVersion, FolderHits};

    use super::*;

    fn plain() -> Output {
        Output::resolve(false, false, true)
    }

    #[test]
    fn folders_end_with_a_slash() {
        let listing = [
            FolderEntry {
                name: "clients".to_owned(),
                kind: EntryKind::Folder,
            },
            FolderEntry {
                name: "start".to_owned(),
                kind: EntryKind::Page,
            },
        ];
        assert_eq!(entries(&listing), "clients/\nstart\n");
    }

    fn hit(kind: HitKind, path: &str, page: Option<u32>, line_number: Option<u32>) -> SearchHit {
        SearchHit {
            kind,
            id: 1,
            path: path.to_owned(),
            page,
            line_number,
            line: Some("fee: 10".to_owned()),
            author: "mino".to_owned(),
            at: "2026-11-24".to_owned(),
        }
    }

    #[test]
    fn search_shows_hits_then_each_folder_with_its_count() {
        let results = SearchResults {
            hits: vec![hit(HitKind::Page, "clients/acme", None, Some(4))],
            folders: vec![FolderHits {
                path: "clients/archive".to_owned(),
                matches: 12,
                hits: vec![hit(
                    HitKind::File,
                    "clients/archive/a.pdf",
                    Some(2),
                    Some(1),
                )],
            }],
            next: None,
            no_hits: None,
        };
        assert_eq!(
            search(&results, plain()),
            "clients/acme:4  fee: 10\nclients/archive/  12 matches\n  clients/archive/a.pdf (page 2)  fee: 10\n"
        );
    }

    #[test]
    fn history_shows_each_version_with_its_diff() {
        let list = VersionList {
            versions: vec![VersionView {
                version: 2,
                path: "a".to_owned(),
                op: Operation::Edit,
                author: "mino".to_owned(),
                at: "2026-11-24".to_owned(),
                group: 1,
                reverts_version: None,
                diff: "-old\n+new\n".to_owned(),
            }],
            next: None,
        };
        let text = versions(&list, plain());
        assert!(text.starts_with("v2  edit  2026-11-24  mino\n"));
        assert!(text.contains("  -old\n  +new\n"));
    }

    #[test]
    fn changes_list_each_group_with_its_pages() {
        let list = Changes {
            since: 0,
            last_seq: 3,
            next: None,
            groups: vec![ChangeGroup {
                id: 1,
                task: Some("tidy".to_owned()),
                author: "agent".to_owned(),
                author_display_name: "Agent".to_owned(),
                at: "2026-11-24".to_owned(),
                versions: vec![ChangedVersion {
                    page_id: 1,
                    path: "a".to_owned(),
                    version: 2,
                    op: Operation::Move,
                    seq: 3,
                    diff: String::new(),
                }],
            }],
        };
        assert_eq!(
            changes(&list, plain()),
            "2026-11-24  agent  tidy\n  move a v2\n"
        );
    }
}
