use std::io::{Cursor, Read};

use bytes::Bytes;
use memory_ops::{PagePath, WriteAction};
use serde::Serialize;
use sqlx::PgPool;
use utoipa::ToSchema;

use crate::access::{Actor, Scope};
use crate::files::{FileContent, Files, FilesError, mime_for_path};
use crate::pages::error::PagesError;
use crate::pages::write::write;
use crate::time_zone::TimeZone;
use crate::transfer::layout::{self, Entry};
use crate::transfer::{MAX_ARCHIVE_BYTES, TransferError};

const IMPORT_AUTHOR: &str = "import";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SkipReason {
    Unrecognized,
    InvalidPath,
    PathTaken,
    InvalidPage,
    Credential,
    TooLarge,
    UnsupportedType,
    Empty,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub(crate) struct Skipped {
    pub(crate) name: String,
    pub(crate) reason: SkipReason,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub(crate) struct ImportReport {
    pub(crate) pages: usize,
    pub(crate) files: usize,
    pub(crate) skipped: Vec<Skipped>,
}

#[derive(Debug)]
struct Member {
    name: String,
    data: Vec<u8>,
}

pub(crate) async fn import(
    pool: &PgPool,
    time_zone: &TimeZone,
    files: &Files,
    importer: &Actor,
    archive: Bytes,
) -> Result<ImportReport, TransferError> {
    let members = tokio::task::spawn_blocking(move || unpack(archive))
        .await
        .map_err(|error| TransferError::Write(std::io::Error::other(error)))??;
    let actor = Actor {
        name: IMPORT_AUTHOR.to_owned(),
        scope: Scope::everything(),
        ..importer.clone()
    };
    let mut report = ImportReport::default();
    let (page_members, file_members): (Vec<_>, Vec<_>) = members
        .iter()
        .partition(|member| matches!(layout::classify(&member.name), Some(Entry::Page(_))));
    for member in file_members {
        match layout::classify(&member.name) {
            Some(Entry::File(path)) => {
                match import_file(files, &actor, path, &member.data).await? {
                    None => report.files += 1,
                    Some(reason) => skip(&mut report, &member.name, reason),
                }
            }
            Some(Entry::Page(_)) | None => {
                skip(&mut report, &member.name, SkipReason::Unrecognized);
            }
        }
    }
    for member in page_members {
        let Some(Entry::Page(path)) = layout::classify(&member.name) else {
            continue;
        };
        match import_page(pool, time_zone, &actor, path, &member.data).await? {
            None => report.pages += 1,
            Some(reason) => skip(&mut report, &member.name, reason),
        }
    }
    Ok(report)
}

fn skip(report: &mut ImportReport, name: &str, reason: SkipReason) {
    report.skipped.push(Skipped {
        name: name.to_owned(),
        reason,
    });
}

async fn import_file(
    files: &Files,
    actor: &Actor,
    path: &str,
    data: &[u8],
) -> Result<Option<SkipReason>, TransferError> {
    let Ok(page_path) = PagePath::parse(path) else {
        return Ok(Some(SkipReason::InvalidPath));
    };
    let Some(mime) = mime_for_path(&page_path) else {
        return Ok(Some(SkipReason::UnsupportedType));
    };
    let content = FileContent::Bytes(Bytes::copy_from_slice(data));
    match files.upload(actor, path, content, mime).await {
        Ok(_) => Ok(None),
        Err(error) => file_skip(error).map(Some),
    }
}

fn file_skip(error: FilesError) -> Result<SkipReason, TransferError> {
    match error {
        FilesError::InvalidPath(_) => Ok(SkipReason::InvalidPath),
        FilesError::PathTaken => Ok(SkipReason::PathTaken),
        FilesError::Empty => Ok(SkipReason::Empty),
        FilesError::TooLarge { .. } => Ok(SkipReason::TooLarge),
        FilesError::UnsupportedMime | FilesError::InvalidText | FilesError::Pdf(_) => {
            Ok(SkipReason::UnsupportedType)
        }
        FilesError::Credential(_) => Ok(SkipReason::Credential),
        FilesError::Forbidden
        | FilesError::NotFound
        | FilesError::Purged
        | FilesError::PageOutOfRange { .. }
        | FilesError::ConnectionRequired
        | FilesError::LinkInvalid
        | FilesError::StoredRowInvalid
        | FilesError::Storage(_)
        | FilesError::Database(_) => Err(TransferError::Files(error)),
    }
}

async fn import_page(
    pool: &PgPool,
    time_zone: &TimeZone,
    actor: &Actor,
    path: &str,
    data: &[u8],
) -> Result<Option<SkipReason>, TransferError> {
    let Ok(path) = PagePath::parse(path) else {
        return Ok(Some(SkipReason::InvalidPath));
    };
    let Ok(content) = String::from_utf8(data.to_vec()) else {
        return Ok(Some(SkipReason::InvalidPage));
    };
    let action = WriteAction::Create { content };
    match write(pool, time_zone, actor, &path, action, Some(IMPORT_AUTHOR)).await {
        Ok(_) => Ok(None),
        Err(error) => page_skip(error).map(Some),
    }
}

fn page_skip(error: PagesError) -> Result<SkipReason, TransferError> {
    match error {
        PagesError::PathTaken => Ok(SkipReason::PathTaken),
        PagesError::TooLarge => Ok(SkipReason::TooLarge),
        PagesError::Credential(_) => Ok(SkipReason::Credential),
        PagesError::Grammar(_) | PagesError::ReservedPath => Ok(SkipReason::InvalidPage),
        PagesError::Database(_)
        | PagesError::NotFound
        | PagesError::Purged
        | PagesError::VersionNotFound
        | PagesError::GroupNotFound
        | PagesError::OutOfScope
        | PagesError::ReadOnly
        | PagesError::SamePath
        | PagesError::Conflict { .. }
        | PagesError::TimelineChanged
        | PagesError::Replace(_)
        | PagesError::FutureDate { .. }
        | PagesError::QuoteUnverified { .. }
        | PagesError::NoChange
        | PagesError::AlreadyCurrent
        | PagesError::RevertBlocked(_)
        | PagesError::TaskLength
        | PagesError::StoredPath(_) => Err(TransferError::Pages(error)),
    }
}

fn unpack(archive: Bytes) -> Result<Vec<Member>, TransferError> {
    let mut zip =
        zip::ZipArchive::new(Cursor::new(archive)).map_err(TransferError::InvalidArchive)?;
    let mut members = Vec::new();
    let mut remaining = MAX_ARCHIVE_BYTES;
    for index in 0..zip.len() {
        let entry = zip.by_index(index).map_err(TransferError::InvalidArchive)?;
        if entry.is_dir() {
            continue;
        }
        let name = entry
            .name()
            .map_err(TransferError::InvalidArchive)?
            .into_owned();
        let mut data = Vec::new();
        let read = entry
            .take(remaining.saturating_add(1))
            .read_to_end(&mut data)
            .map_err(TransferError::Write)?;
        remaining = remaining
            .checked_sub(u64::try_from(read).unwrap_or(u64::MAX))
            .ok_or(TransferError::TooLarge)?;
        members.push(Member { name, data });
    }
    Ok(members)
}
