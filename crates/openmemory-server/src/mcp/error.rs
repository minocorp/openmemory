use openmemory_ops::{BlockReason, BlockedPage, PagePathError, RevertInputError, WriteInputError};

use crate::files::FilesError;
use crate::pages::error::{PagesError, QuoteProblem};
use crate::search::SearchError;

#[derive(Debug, thiserror::Error)]
pub(super) enum ToolError {
    #[error("tool call rejected")]
    Rejected(String),
    #[error("tool call failed")]
    Internal(#[source] Box<dyn std::error::Error + Send + Sync>),
}

impl ToolError {
    pub(super) fn rejected(message: impl Into<String>) -> Self {
        Self::Rejected(message.into())
    }
}

impl From<PagePathError> for ToolError {
    fn from(error: PagePathError) -> Self {
        Self::rejected(format!(
            "Invalid path: {error}. Paths look like `customers/acme`: no leading slash, no empty segments, no `..`."
        ))
    }
}

impl From<WriteInputError> for ToolError {
    fn from(error: WriteInputError) -> Self {
        Self::rejected(format!(
            "Invalid write: {error}. Valid calls: `content` alone creates a page; `content` with `base_version` replaces a whole page; `replace_old` with `replace_new` and/or `append_text` (with optional `append_source` and `append_date`) edits one."
        ))
    }
}

impl From<RevertInputError> for ToolError {
    fn from(error: RevertInputError) -> Self {
        Self::rejected(format!(
            "Invalid revert: {error}. Get versions and groups from `history`."
        ))
    }
}

impl From<SearchError> for ToolError {
    fn from(error: SearchError) -> Self {
        match error {
            SearchError::Database(_) => Self::Internal(Box::new(error)),
            SearchError::Where(ref reason) => Self::rejected(format!(
                "Invalid `where`: {reason}. Each entry is one condition such as `status=active`, `renewal<+90d`, `owner!=kim`, `note~late` or `renewal:exists`."
            )),
            SearchError::Folder(_) => Self::rejected(
                "Invalid `folder`: use a folder path such as `customers`, without a leading slash.",
            ),
        }
    }
}

impl From<FilesError> for ToolError {
    fn from(error: FilesError) -> Self {
        match error {
            FilesError::StoredRowInvalid | FilesError::Storage(_) | FilesError::Database(_) => {
                Self::Internal(Box::new(error))
            }
            FilesError::InvalidPath(reason) => reason.into(),
            FilesError::Forbidden => Self::rejected(
                "Not allowed to write this path: it is outside this connection's folder scope, or the connection is read-only. Ask the owner to change the connection in Settings.",
            ),
            FilesError::NotFound => Self::rejected(
                "No file at this path, or it is outside this connection's folder scope.",
            ),
            FilesError::PathTaken => {
                Self::rejected("A page or file already exists at this path. Choose another path.")
            }
            FilesError::Empty => Self::rejected("The file is empty."),
            FilesError::TooLarge { limit } => {
                Self::rejected(format!("The file is over the {limit} byte limit."))
            }
            FilesError::UnsupportedMime => Self::rejected(
                "This file type is not accepted. Use a pdf, text, markdown, csv, image, spreadsheet or audio extension.",
            ),
            FilesError::InvalidText => Self::rejected("The text file is not valid UTF-8."),
            FilesError::Credential(pattern) => Self::rejected(format!(
                "The text matches a secret pattern ({}), so nothing was saved. Remove the secret and upload again.",
                pattern.name()
            )),
            FilesError::PageOutOfRange { page } => {
                Self::rejected(format!("Page {page} does not exist in this file."))
            }
            FilesError::ConnectionRequired | FilesError::LinkInvalid => {
                Self::rejected("An upload link could not be created for this connection.")
            }
            FilesError::Purged => Self::rejected("This file was purged and no longer exists."),
            FilesError::Pdf(_) => Self::rejected(
                "Text could not be extracted from this PDF. Check that it is a valid PDF without a password.",
            ),
        }
    }
}

impl From<PagesError> for ToolError {
    fn from(error: PagesError) -> Self {
        match error {
            PagesError::Database(_) | PagesError::StoredPath(_) => Self::Internal(Box::new(error)),
            PagesError::NotFound => Self::rejected(
                "No page at this path, or it is outside this connection's folder scope. `read` the parent folder or `/` to see what exists, or `search` for it.",
            ),
            PagesError::VersionNotFound => Self::rejected(
                "That version does not exist for this page. Call `history` with the page path to list its versions.",
            ),
            PagesError::GroupNotFound => Self::rejected(
                "No change group with that number, or it is outside this connection's folder scope. Call `history` to find group numbers.",
            ),
            PagesError::OutOfScope => Self::rejected(
                "This path is outside this connection's folder scope. Ask the owner to widen the scope in Settings.",
            ),
            PagesError::ReadOnly => Self::rejected(
                "This connection is read-only. Ask the owner to turn on writing for it in Settings.",
            ),
            PagesError::ReservedPath => Self::rejected(
                "The `start` page cannot be moved or deleted. Change it with `write`.",
            ),
            PagesError::PathTaken => Self::rejected(
                "A page or file already exists at the target path. Choose another path, or `read` the existing one.",
            ),
            PagesError::SamePath => Self::rejected("`from` and `to` are the same path."),
            PagesError::Conflict { current_version } => Self::rejected(format!(
                "The page changed since the version you based this on; it is now at version {}. `read` it again, apply your change to the new text, and write again with `base_version` {}.",
                current_version.0, current_version.0
            )),
            PagesError::TooLarge => Self::rejected(
                "The page would be over the 1 MB limit. Split the content into several pages.",
            ),
            PagesError::Credential(pattern) => Self::rejected(format!(
                "The text matches a {} pattern, so nothing was saved. Remove it and write again.",
                pattern.name()
            )),
            PagesError::Grammar(ref reason) => Self::rejected(format!(
                "The page breaks the page format at {reason}. Frontmatter is flat YAML between `---` lines; timeline lines are `YYYY-MM-DD | source — text` under the last `## Timeline` heading."
            )),
            PagesError::TimelineChanged => Self::rejected(
                "Existing timeline lines cannot be changed. Use `append_text` to add a line; `revert` restores an older timeline.",
            ),
            PagesError::Replace(ref reason) => Self::rejected(format!(
                "Replace failed: {reason}. `old` must match the page text exactly once, so `read` the page and include more surrounding text if it matches several times."
            )),
            PagesError::FutureDate { today } => Self::rejected(format!(
                "The timeline date is after today ({}). Use the day the fact happened, or leave `append_date` out for today.",
                openmemory_ops::format_iso_date(today)
            )),
            PagesError::QuoteUnverified {
                problem,
                ref path,
                page,
            } => Self::rejected(quote_message(problem, path, page)),
            PagesError::NoChange => Self::rejected("This write would change nothing."),
            PagesError::AlreadyCurrent => {
                Self::rejected("The page is already in the state of that version.")
            }
            PagesError::RevertBlocked(blocked) => Self::rejected(blocked_message(&blocked)),
            PagesError::TaskLength => Self::rejected("`task` must be 1 to 200 characters."),
            PagesError::Purged => Self::rejected("This page was purged and no longer exists."),
        }
    }
}

fn quote_message(problem: QuoteProblem, path: &str, page: Option<u32>) -> String {
    let place = page.map_or_else(
        || format!("`{path}`"),
        |page| format!("page {page} of `{path}`"),
    );
    match problem {
        QuoteProblem::NoFile => format!(
            "The source quotes `{path}`, but no file exists there in your scope. Check the path with `read`, upload the file first, or cite the source without a quote."
        ),
        QuoteProblem::NoPage => format!(
            "The source quotes {place}, but the file has no such page. `read` the file to see its page count."
        ),
        QuoteProblem::NoText => format!(
            "The source quotes {place}, which has no extracted text (a scan or image), so the quote cannot be checked. Cite it without a quote, or write a transcript, upload it as a text file and quote that."
        ),
        QuoteProblem::NotInText => format!(
            "The quote does not occur in {place}. `read` that page again and copy the words exactly; case and whitespace are ignored, wording is not. Shorten the quote if the page wraps lines oddly."
        ),
    }
}

fn blocked_message(blocked: &[BlockedPage]) -> String {
    let pages = blocked
        .iter()
        .map(|page| {
            let reason = match page.reason {
                BlockReason::ChangedSince => "changed after the group",
                BlockReason::PathTaken => "its old path is now taken",
            };
            format!("{} ({reason})", page.path)
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Nothing was reverted because these pages block it: {pages}. Revert single page versions instead, or resolve the blocking pages first."
    )
}
