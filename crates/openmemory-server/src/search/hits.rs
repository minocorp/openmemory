use std::collections::HashMap;

use openmemory_ops::{FieldValue, Page, SearchHit};

use crate::files::FileId;
use crate::pages::ids::PageId;
use crate::pages::read::format_time;
use crate::search::candidate::Candidate;
use crate::search::position::Document;
use crate::search::queries::texts::{FilePageText, PageText};
use crate::search::terms::Terms;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FoundHit {
    pub(crate) document: Document,
    pub(crate) hit: SearchHit,
}

#[derive(Debug, Default)]
pub(super) struct Texts {
    pages: HashMap<PageId, String>,
    files: HashMap<FileId, (u32, String)>,
}

impl Texts {
    pub(super) fn new(pages: Vec<PageText>, files: Vec<FilePageText>) -> Self {
        Self {
            pages: pages
                .into_iter()
                .map(|page| (page.id, page.content))
                .collect(),
            files: files
                .into_iter()
                .map(|file| {
                    let page = u32::try_from(file.page).unwrap_or(1);
                    (file.file_id, (page, file.text))
                })
                .collect(),
        }
    }

    pub(super) fn hits_for(
        &self,
        candidate: &Candidate,
        terms: &Terms,
        max_lines: usize,
    ) -> Vec<FoundHit> {
        let (page, text) = match candidate.document {
            Document::Page(id) => (None, self.pages.get(&id)),
            Document::File(id) => match self.files.get(&id) {
                Some((page, text)) => (Some(*page), Some(text)),
                None => (None, None),
            },
        };
        let text = text.map_or("", String::as_str);
        let lines = terms.matching_lines(text, max_lines);
        if lines.is_empty() {
            let line = match candidate.document {
                Document::Page(_) => fields_line(text),
                Document::File(_) => None,
            };
            return vec![found(candidate, page, None, line)];
        }
        lines
            .into_iter()
            .map(|(number, line)| found(candidate, page, Some(number), Some(line)))
            .collect()
    }
}

fn found(
    candidate: &Candidate,
    page: Option<u32>,
    line_number: Option<u32>,
    line: Option<String>,
) -> FoundHit {
    FoundHit {
        document: candidate.document,
        hit: SearchHit {
            kind: candidate.document.kind(),
            path: candidate.path.clone(),
            page,
            line_number,
            line,
            author: candidate.author.clone(),
            at: format_time(candidate.at),
        },
    }
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
