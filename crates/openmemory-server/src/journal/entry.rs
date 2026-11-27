use openmemory_ops::{PagePath, VersionOp};

use crate::access::Scope;
use crate::files::FileId;
use crate::journal::JournalError;
use crate::journal::queries::EntryRow;
use crate::pages::ids::{PageId, Seq, VersionNumber};
use crate::purge::Subject;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileOp {
    Upload,
    Delete,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Change {
    Page {
        id: PageId,
        version: VersionNumber,
        op: VersionOp,
        path: Option<PagePath>,
        previous_path: Option<PagePath>,
    },
    File {
        id: FileId,
        op: FileOp,
        path: PagePath,
    },
    Purge {
        subject: Subject,
        id: i64,
        path: PagePath,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Entry {
    pub(super) seq: Seq,
    pub(super) change: Change,
}

impl FileOp {
    fn parse(text: &str) -> Option<Self> {
        match text {
            "upload" => Some(Self::Upload),
            "delete" => Some(Self::Delete),
            _ => None,
        }
    }
}

impl Change {
    pub(super) fn visible_to(self, scope: &Scope) -> Option<Self> {
        let readable = |path: Option<PagePath>| path.filter(|path| scope.can_read(path));
        match self {
            Self::Page {
                id,
                version,
                op,
                path,
                previous_path,
            } => {
                let path = readable(path);
                let previous_path = readable(previous_path);
                (path.is_some() || previous_path.is_some()).then_some(Self::Page {
                    id,
                    version,
                    op,
                    path,
                    previous_path,
                })
            }
            Self::File { ref path, .. } | Self::Purge { ref path, .. } => {
                scope.can_read(path).then_some(self)
            }
        }
    }
}

impl TryFrom<EntryRow> for Entry {
    type Error = JournalError;

    fn try_from(row: EntryRow) -> Result<Self, Self::Error> {
        let change = page(&row)
            .or_else(|| file(&row))
            .or_else(|| purge(&row))
            .ok_or(JournalError::StoredRowInvalid)?;
        Ok(Self {
            seq: row.seq,
            change,
        })
    }
}

fn page(row: &EntryRow) -> Option<Change> {
    let path = parse_path(row.page_path.as_deref()?)?;
    let previous_path = match row.previous_path.as_deref() {
        Some(previous) if previous != path.as_str() => Some(parse_path(previous)?),
        Some(_) | None => None,
    };
    Some(Change::Page {
        id: row.page_id?,
        version: row.n?,
        op: VersionOp::parse(row.page_op.as_deref()?)?,
        path: Some(path),
        previous_path,
    })
}

fn file(row: &EntryRow) -> Option<Change> {
    Some(Change::File {
        id: row.file_id?,
        op: FileOp::parse(row.file_op.as_deref()?)?,
        path: parse_path(row.file_path.as_deref()?)?,
    })
}

fn purge(row: &EntryRow) -> Option<Change> {
    Some(Change::Purge {
        subject: Subject::parse(row.purge_kind.as_deref()?)?,
        id: row.purge_subject_id?,
        path: parse_path(row.purge_path.as_deref()?)?,
    })
}

fn parse_path(text: &str) -> Option<PagePath> {
    PagePath::parse(text).ok()
}
