use std::fmt::Write;

use memory_api::{
    Changes, EntryKind, FolderEntry, Operation, SearchResults, VersionList, VersionView,
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
    for hit in &results.pages {
        let place = hit
            .line_number
            .map_or_else(String::new, |line| format!(":{line}"));
        let line = hit.line.as_deref().unwrap_or_default();
        let _ = writeln!(text, "{}{place}  {line}", output.bold(&hit.path));
    }
    for hit in &results.files {
        let page = hit
            .page
            .map_or_else(String::new, |page| format!(" (page {page})"));
        let line = hit.line.as_deref().unwrap_or_default();
        let _ = writeln!(text, "{}{page}  {line}", output.bold(&hit.path));
    }
    text
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
    use memory_api::{ChangeGroup, ChangedVersion, PageHit};

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

    #[test]
    fn search_hits_show_path_line_and_text() {
        let results = SearchResults {
            pages: vec![PageHit {
                page_id: 1,
                path: "clients/acme".to_owned(),
                line_number: Some(4),
                line: Some("fee: 10".to_owned()),
                author: "mino".to_owned(),
                at: "2026-11-24".to_owned(),
            }],
            files: Vec::new(),
            next: None,
            no_hits: None,
        };
        assert_eq!(search(&results, plain()), "clients/acme:4  fee: 10\n");
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
