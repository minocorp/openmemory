use axum::http::StatusCode;
use axum::http::header::ETAG;
use axum::response::{IntoResponse, Response};
use openmemory_api::{PageView, Problem};
use openmemory_ops::{BlockReason, PageErrorKind, ReplaceError};
use serde_json::json;

use crate::api::precondition::etag;
use crate::auth::AuthError;
use crate::files::FilesError;
use crate::pages::error::{PagesError, QuoteProblem};
use crate::pages::ids::VersionNumber;
use crate::problem::respond;
use crate::search::SearchError;
use crate::transfer::TransferError;
use crate::tree::TreeError;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ApiError {
    #[error("authentication failed")]
    Auth(#[from] AuthError),
    #[error("page request failed")]
    Pages(#[from] PagesError),
    #[error("file request failed")]
    Files(#[from] FilesError),
    #[error("search failed")]
    Search(#[from] SearchError),
    #[error("transfer failed")]
    Transfer(#[from] TransferError),
    #[error("tree failed")]
    Tree(#[from] TreeError),
    #[error("precondition required")]
    PreconditionRequired,
    #[error("precondition failed")]
    PreconditionFailed {
        version: VersionNumber,
        current: Option<Box<PageView>>,
    },
    #[error("invalid input")]
    InvalidInput(&'static str),
    #[error("file has no text pages")]
    NoTextPages,
    #[error("too many uploads in flight")]
    Busy,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            Self::Auth(error) => error.into_response(),
            Self::PreconditionFailed { version, current } => {
                let mut problem = Problem::new("precondition_failed", 412)
                    .with_detail("the page changed since it was read");
                if let Some(page) = current {
                    problem =
                        problem.with("current", serde_json::to_value(&*page).unwrap_or_default());
                }
                let mut response = respond(problem);
                response.headers_mut().insert(ETAG, etag(version));
                response
            }
            other => {
                let problem = other.describe();
                if problem.status == StatusCode::INTERNAL_SERVER_ERROR.as_u16() {
                    log_internal(&other);
                }
                respond(problem)
            }
        }
    }
}

impl ApiError {
    fn describe(&self) -> Problem {
        match self {
            Self::Pages(error) | Self::Tree(TreeError::Pages(error)) => describe_pages(error),
            Self::Files(error) | Self::Tree(TreeError::Files(error)) => describe_files(error),
            Self::Search(error) => describe_search(error),
            Self::Transfer(error) => describe_transfer(error),
            Self::PreconditionRequired => {
                plain(StatusCode::PRECONDITION_REQUIRED, "precondition_required")
            }
            Self::InvalidInput(field) => invalid_input(field),
            Self::NoTextPages => plain(StatusCode::UNPROCESSABLE_ENTITY, "no_text_pages"),
            Self::Busy => plain(StatusCode::SERVICE_UNAVAILABLE, "busy"),
            Self::Auth(_) | Self::PreconditionFailed { .. } => internal(),
        }
    }
}

fn internal() -> Problem {
    plain(StatusCode::INTERNAL_SERVER_ERROR, "internal")
}

fn plain(status: StatusCode, code: &str) -> Problem {
    Problem::new(code, status.as_u16())
}

fn invalid_input(field: &str) -> Problem {
    plain(StatusCode::BAD_REQUEST, "invalid_input").with("field", field)
}

fn describe_pages(error: &PagesError) -> Problem {
    match error {
        PagesError::NotFound => plain(StatusCode::NOT_FOUND, "not_found"),
        PagesError::VersionNotFound => plain(StatusCode::NOT_FOUND, "version_not_found"),
        PagesError::GroupNotFound => plain(StatusCode::NOT_FOUND, "group_not_found"),
        PagesError::Purged => plain(StatusCode::GONE, "purged"),
        PagesError::OutOfScope => plain(StatusCode::FORBIDDEN, "out_of_scope"),
        PagesError::ReadOnly => plain(StatusCode::FORBIDDEN, "read_only"),
        PagesError::ReservedPath => plain(StatusCode::BAD_REQUEST, "reserved_path"),
        PagesError::SamePath => plain(StatusCode::BAD_REQUEST, "same_path"),
        PagesError::TaskLength => invalid_input("task"),
        PagesError::PathTaken => plain(StatusCode::CONFLICT, "path_taken"),
        PagesError::AlreadyCurrent => plain(StatusCode::CONFLICT, "already_current"),
        PagesError::Conflict { .. } => {
            plain(StatusCode::PRECONDITION_FAILED, "precondition_failed")
        }
        PagesError::TooLarge => plain(StatusCode::PAYLOAD_TOO_LARGE, "too_large"),
        PagesError::Credential(pattern) => {
            plain(StatusCode::UNPROCESSABLE_ENTITY, "credential").with("pattern", pattern.name())
        }
        PagesError::Grammar(error) => plain(StatusCode::UNPROCESSABLE_ENTITY, "grammar")
            .with("line", error.line())
            .with("reason", grammar_reason(error.kind())),
        PagesError::TimelineChanged => plain(StatusCode::UNPROCESSABLE_ENTITY, "timeline_changed"),
        PagesError::Replace(error) => plain(StatusCode::UNPROCESSABLE_ENTITY, "replace_failed")
            .with("reason", replace_reason(error)),
        PagesError::FutureDate { today } => plain(StatusCode::UNPROCESSABLE_ENTITY, "future_date")
            .with("today", openmemory_ops::format_iso_date(*today)),
        PagesError::QuoteUnverified { problem, .. } => {
            plain(StatusCode::UNPROCESSABLE_ENTITY, "quote_unverified")
                .with("reason", quote_reason(*problem))
        }
        PagesError::NoChange => plain(StatusCode::UNPROCESSABLE_ENTITY, "no_change"),
        PagesError::RevertBlocked(blocked) => plain(StatusCode::CONFLICT, "revert_blocked").with(
            "blocked",
            blocked
                .iter()
                .map(|page| json!({ "path": page.path, "reason": block_reason(page.reason) }))
                .collect::<Vec<_>>(),
        ),
        PagesError::Database(_) | PagesError::StoredPath(_) => internal(),
    }
}

fn quote_reason(problem: QuoteProblem) -> &'static str {
    match problem {
        QuoteProblem::NoFile => "no_file",
        QuoteProblem::NoPage => "no_page",
        QuoteProblem::NoText => "no_text",
        QuoteProblem::NotInText => "not_in_text",
    }
}

fn grammar_reason(kind: &PageErrorKind) -> &'static str {
    match kind {
        PageErrorKind::UnclosedFrontmatter => "unclosed_frontmatter",
        PageErrorKind::Frontmatter { .. } => "frontmatter",
        PageErrorKind::FieldNotScalar { .. } => "field_not_scalar",
        PageErrorKind::FieldKeyLength { .. } => "field_key_length",
        PageErrorKind::FieldValueTooLong { .. } => "field_value_too_long",
        PageErrorKind::TimelineDate => "timeline_date",
        PageErrorKind::TimelineSeparator => "timeline_separator",
        PageErrorKind::TimelineSource => "timeline_source",
        PageErrorKind::TimelineText => "timeline_text",
        PageErrorKind::TimelineBlankLine => "timeline_blank_line",
    }
}

fn replace_reason(error: &ReplaceError) -> &'static str {
    match error {
        ReplaceError::EmptyOld => "empty",
        ReplaceError::NotFound => "not_found",
        ReplaceError::Ambiguous { .. } => "ambiguous",
    }
}

fn block_reason(reason: BlockReason) -> &'static str {
    match reason {
        BlockReason::ChangedSince => "changed_since",
        BlockReason::PathTaken => "path_taken",
    }
}

fn describe_files(error: &FilesError) -> Problem {
    match error {
        FilesError::InvalidPath(_) => invalid_input("path"),
        FilesError::Forbidden => plain(StatusCode::FORBIDDEN, "forbidden"),
        FilesError::NotFound => plain(StatusCode::NOT_FOUND, "not_found"),
        FilesError::Purged => plain(StatusCode::GONE, "purged"),
        FilesError::PathTaken => plain(StatusCode::CONFLICT, "path_taken"),
        FilesError::Empty => plain(StatusCode::BAD_REQUEST, "empty"),
        FilesError::TooLarge { limit } => {
            plain(StatusCode::PAYLOAD_TOO_LARGE, "too_large").with("limit", *limit)
        }
        FilesError::UnsupportedMime => {
            plain(StatusCode::UNSUPPORTED_MEDIA_TYPE, "unsupported_type")
        }
        FilesError::InvalidText => plain(StatusCode::UNPROCESSABLE_ENTITY, "invalid_text"),
        FilesError::Credential(pattern) => {
            plain(StatusCode::UNPROCESSABLE_ENTITY, "credential").with("pattern", pattern.name())
        }
        FilesError::PageOutOfRange { .. } => plain(StatusCode::NOT_FOUND, "page_not_found"),
        FilesError::ConnectionRequired => plain(StatusCode::BAD_REQUEST, "connection_required"),
        FilesError::LinkInvalid => plain(StatusCode::NOT_FOUND, "link_invalid"),
        FilesError::Pdf(_) => plain(StatusCode::UNPROCESSABLE_ENTITY, "unreadable_pdf"),
        FilesError::StoredRowInvalid | FilesError::Storage(_) | FilesError::Database(_) => {
            internal()
        }
    }
}

fn describe_search(error: &SearchError) -> Problem {
    match error {
        SearchError::Where(_) => plain(StatusCode::BAD_REQUEST, "invalid_where"),
        SearchError::Folder(_) => invalid_input("folder"),
        SearchError::Database(_) => internal(),
    }
}

fn describe_transfer(error: &TransferError) -> Problem {
    match error {
        TransferError::InvalidArchive(_) => plain(StatusCode::BAD_REQUEST, "invalid_archive"),
        TransferError::TooLarge => plain(StatusCode::PAYLOAD_TOO_LARGE, "too_large"),
        TransferError::Pages(error) => describe_pages(error),
        TransferError::Files(error) => describe_files(error),
        TransferError::Write(_) => internal(),
    }
}

fn log_internal(error: &ApiError) {
    let mut message = error.to_string();
    let mut source = std::error::Error::source(error);
    while let Some(cause) = source {
        message = format!("{message}: {cause}");
        source = cause.source();
    }
    tracing::error!(error = %message, "request failed");
}
