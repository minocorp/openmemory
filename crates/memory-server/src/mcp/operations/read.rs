use memory_ops::{
    Envelope, FileRead, FolderEntry, FolderRead, Origin, PagePath, PagePathError, ReadInput,
    ReadOutput,
};

use crate::access::Actor;
use crate::files::{FileRecord, FileRef, FilesError};
use crate::mcp::error::ToolError;
use crate::pages::error::PagesError;
use crate::pages::ids::VersionNumber;
use crate::pages::{folder, read};
use crate::state::AppState;

enum Target {
    Folder(Option<PagePath>),
    Entry(PagePath),
}

impl Target {
    fn parse(input: &str) -> Result<Self, PagePathError> {
        if input.is_empty() || input == "/" {
            return Ok(Self::Folder(None));
        }
        match input.strip_suffix('/') {
            Some(folder) => Ok(Self::Folder(Some(PagePath::parse(folder)?))),
            None => Ok(Self::Entry(PagePath::parse(input)?)),
        }
    }
}

pub(in crate::mcp) async fn read(
    state: &AppState,
    actor: &Actor,
    input: ReadInput,
) -> Result<Envelope<ReadOutput>, ToolError> {
    match Target::parse(&input.path)? {
        Target::Folder(path) => list(state, actor, path.as_ref()).await,
        Target::Entry(path) => entry(state, actor, &path, &input).await,
    }
}

async fn entry(
    state: &AppState,
    actor: &Actor,
    path: &PagePath,
    input: &ReadInput,
) -> Result<Envelope<ReadOutput>, ToolError> {
    let version = input.version.map(VersionNumber);
    match read::read(state.pool(), actor, path, version).await {
        Ok(page) => {
            let origin = Origin::of_author(&page.author);
            return Ok(Envelope::new(origin, ReadOutput::Page(page)));
        }
        Err(PagesError::NotFound) => {}
        Err(error) => return Err(error.into()),
    }
    match state
        .files()
        .metadata(actor, &FileRef::Path(path.clone()))
        .await
    {
        Ok(record) => file(state, actor, record, input.page).await,
        Err(FilesError::NotFound) => list(state, actor, Some(path)).await,
        Err(error) => Err(error.into()),
    }
}

async fn file(
    state: &AppState,
    actor: &Actor,
    record: FileRecord,
    page: Option<u32>,
) -> Result<Envelope<ReadOutput>, ToolError> {
    let page_count = u32::try_from(record.page_count).unwrap_or(u32::MAX);
    let wanted = if page_count == 0 {
        if page.is_some() {
            return Err(ToolError::rejected(
                "This file has no extracted text pages: only PDF and text files have them. Images, spreadsheets and audio need a transcript page written by a person or agent.",
            ));
        }
        None
    } else {
        Some(page.unwrap_or(1))
    };
    let text = match wanted {
        Some(number) => Some(
            state
                .files()
                .page_text(actor, &FileRef::Id(record.id), number)
                .await
                .map_err(|error| out_of_range(error, page_count))?,
        ),
        None => None,
    };
    let output = FileRead {
        path: record.path.to_string(),
        mime: record.mime,
        size: record.size,
        page_count,
        page: wanted,
        text,
    };
    Ok(Envelope::new(Origin::Extracted, ReadOutput::File(output)))
}

fn out_of_range(error: FilesError, page_count: u32) -> ToolError {
    match error {
        FilesError::PageOutOfRange { page } => ToolError::rejected(format!(
            "Page {page} does not exist: this file has {page_count} text pages, numbered from 1."
        )),
        other => other.into(),
    }
}

async fn list(
    state: &AppState,
    actor: &Actor,
    path: Option<&PagePath>,
) -> Result<Envelope<ReadOutput>, ToolError> {
    let mut entries: Vec<FolderEntry> = folder::entries(state.pool(), actor, path).await?;
    for entry in state.files().entries(actor, path).await? {
        if !entries.contains(&entry) {
            entries.push(entry);
        }
    }
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    if entries.is_empty()
        && let Some(missing) = path
    {
        return Err(ToolError::rejected(format!(
            "Nothing at `{missing}`: no page, file or folder here, or it is outside this connection's folder scope. `read` the parent folder or `/` to see what exists, or `search` for it."
        )));
    }
    let output = FolderRead {
        path: path.map_or_else(|| "/".to_owned(), ToString::to_string),
        entries,
    };
    Ok(Envelope::new(Origin::Mixed, ReadOutput::Folder(output)))
}
