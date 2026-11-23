use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::page::{Page, Source, TimelineEntry};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = crate::schema::omit_null)]
pub struct ReadInput {
    #[schemars(
        description = "Path of a page, file or folder, for example `customers/acme`, `contracts/acme.pdf` or `customers`. A folder path lists its entries; `/` lists the top level. Paths have no leading slash and no `..`."
    )]
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Page only: read this older version number instead of the current one. Get version numbers from `history`."
    )]
    pub version: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "File only: number of the PDF or text page to return, starting at 1. Defaults to 1 when the file has text."
    )]
    pub page: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReadOutput {
    Page(PageRead),
    Folder(FolderRead),
    File(FileRead),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PageRead {
    pub path: String,
    pub version: i32,
    pub author: String,
    pub updated_at: String,
    pub fields: Vec<FieldRead>,
    pub body: String,
    pub timeline: Vec<TimelineRead>,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FieldRead {
    pub key: String,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimelineRead {
    pub date: String,
    pub source: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file: Option<FileSourceRead>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileSourceRead {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quote: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FolderRead {
    pub path: String,
    pub entries: Vec<FolderEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FolderEntry {
    pub name: String,
    pub kind: EntryKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EntryKind {
    Page,
    File,
    Folder,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileRead {
    pub path: String,
    pub mime: String,
    pub size: i64,
    pub page_count: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
}

impl PageRead {
    pub fn new(path: &str, version: i32, author: &str, updated_at: &str, page: &Page) -> Self {
        let mut entries: Vec<&TimelineEntry> = page.timeline().iter().collect();
        entries.sort_by_key(|entry| entry.date());
        Self {
            path: path.to_owned(),
            version,
            author: author.to_owned(),
            updated_at: updated_at.to_owned(),
            fields: page
                .fields()
                .iter()
                .map(|field| FieldRead {
                    key: field.key().to_owned(),
                    values: field
                        .values()
                        .iter()
                        .map(|value| value.text().to_owned())
                        .collect(),
                })
                .collect(),
            body: page.body().to_owned(),
            timeline: entries
                .into_iter()
                .map(|entry| TimelineRead {
                    date: crate::format_iso_date(entry.date()),
                    source: entry.source().to_string(),
                    file: match entry.source() {
                        Source::Text(_) => None,
                        Source::File { path, page, quote } => Some(FileSourceRead {
                            path: path.to_string(),
                            page: *page,
                            quote: quote.clone(),
                        }),
                    },
                    text: entry.text().to_owned(),
                })
                .collect(),
            content: page.to_text(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timeline_reads_in_date_order_with_ties_in_append_order() {
        let page = Page::parse(
            "## Timeline\n2026-11-20 | call — b\n2026-03-02 | memo — a\n2026-11-20 | mail — c\n",
        )
        .unwrap();
        let read = PageRead::new("p", 1, "me", "now", &page);
        let texts: Vec<&str> = read
            .timeline
            .iter()
            .map(|entry| entry.text.as_str())
            .collect();
        assert_eq!(texts, ["a", "b", "c"]);
        assert!(read.content.starts_with("## Timeline\n2026-11-20"));
    }
}
