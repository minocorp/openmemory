use axum::Json;
use axum::http::StatusCode;
use axum::http::header::ETAG;
use axum::response::{IntoResponse, Response};
use memory_ops::{BlockReason, PageErrorKind, ReplaceError};
use serde_json::{Value, json};

use crate::api::pages::PageView;
use crate::api::precondition::etag;
use crate::auth::AuthError;
use crate::files::FilesError;
use crate::pages::error::{PagesError, QuoteProblem};
use crate::pages::ids::VersionNumber;
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
    #[error("too many uploads in flight")]
    Busy,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            Self::Auth(error) => error.into_response(),
            Self::PreconditionFailed { version, current } => {
                let body = json!({ "error": "precondition_failed", "current": current });
                let mut response = (StatusCode::PRECONDITION_FAILED, Json(body)).into_response();
                response.headers_mut().insert(ETAG, etag(version));
                response
            }
            other => {
                let (status, body) = other.describe();
                if status == StatusCode::INTERNAL_SERVER_ERROR {
                    log_internal(&other);
                }
                (status, Json(body)).into_response()
            }
        }
    }
}

impl ApiError {
    fn describe(&self) -> (StatusCode, Value) {
        match self {
            Self::Pages(error) | Self::Tree(TreeError::Pages(error)) => describe_pages(error),
            Self::Files(error) | Self::Tree(TreeError::Files(error)) => describe_files(error),
            Self::Search(error) => describe_search(error),
            Self::Transfer(error) => describe_transfer(error),
            Self::PreconditionRequired => (
                StatusCode::PRECONDITION_REQUIRED,
                json!({ "error": "precondition_required" }),
            ),
            Self::InvalidInput(field) => (
                StatusCode::BAD_REQUEST,
                json!({ "error": "invalid_input", "field": field }),
            ),
            Self::Busy => plain(StatusCode::SERVICE_UNAVAILABLE, "busy"),
            Self::Auth(_) | Self::PreconditionFailed { .. } => internal(),
        }
    }
}

fn internal() -> (StatusCode, Value) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        json!({ "error": "internal" }),
    )
}

fn plain(status: StatusCode, code: &str) -> (StatusCode, Value) {
    (status, json!({ "error": code }))
}

fn describe_pages(error: &PagesError) -> (StatusCode, Value) {
    match error {
        PagesError::NotFound => plain(StatusCode::NOT_FOUND, "not_found"),
        PagesError::VersionNotFound => plain(StatusCode::NOT_FOUND, "version_not_found"),
        PagesError::GroupNotFound => plain(StatusCode::NOT_FOUND, "group_not_found"),
        PagesError::Purged => plain(StatusCode::GONE, "purged"),
        PagesError::OutOfScope => plain(StatusCode::FORBIDDEN, "out_of_scope"),
        PagesError::ReadOnly => plain(StatusCode::FORBIDDEN, "read_only"),
        PagesError::ReservedPath => plain(StatusCode::BAD_REQUEST, "reserved_path"),
        PagesError::SamePath => plain(StatusCode::BAD_REQUEST, "same_path"),
        PagesError::TaskLength => (
            StatusCode::BAD_REQUEST,
            json!({ "error": "invalid_input", "field": "task" }),
        ),
        PagesError::PathTaken => plain(StatusCode::CONFLICT, "path_taken"),
        PagesError::AlreadyCurrent => plain(StatusCode::CONFLICT, "already_current"),
        PagesError::Conflict { .. } => {
            plain(StatusCode::PRECONDITION_FAILED, "precondition_failed")
        }
        PagesError::TooLarge => plain(StatusCode::PAYLOAD_TOO_LARGE, "too_large"),
        PagesError::Credential(pattern) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({ "error": "credential", "pattern": pattern.name() }),
        ),
        PagesError::Grammar(error) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({
                "error": "grammar",
                "line": error.line(),
                "reason": grammar_reason(error.kind()),
            }),
        ),
        PagesError::TimelineChanged => plain(StatusCode::UNPROCESSABLE_ENTITY, "timeline_changed"),
        PagesError::Replace(error) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({ "error": "replace_failed", "reason": replace_reason(error) }),
        ),
        PagesError::FutureDate { today } => (
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({ "error": "future_date", "today": memory_ops::format_iso_date(*today) }),
        ),
        PagesError::QuoteUnverified { problem, .. } => (
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({ "error": "quote_unverified", "reason": quote_reason(*problem) }),
        ),
        PagesError::NoChange => plain(StatusCode::UNPROCESSABLE_ENTITY, "no_change"),
        PagesError::RevertBlocked(blocked) => (
            StatusCode::CONFLICT,
            json!({
                "error": "revert_blocked",
                "blocked": blocked
                    .iter()
                    .map(|page| json!({ "path": page.path, "reason": block_reason(page.reason) }))
                    .collect::<Vec<_>>(),
            }),
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

fn describe_files(error: &FilesError) -> (StatusCode, Value) {
    match error {
        FilesError::InvalidPath(_) => (
            StatusCode::BAD_REQUEST,
            json!({ "error": "invalid_input", "field": "path" }),
        ),
        FilesError::Forbidden => plain(StatusCode::FORBIDDEN, "forbidden"),
        FilesError::NotFound => plain(StatusCode::NOT_FOUND, "not_found"),
        FilesError::Purged => plain(StatusCode::GONE, "purged"),
        FilesError::PathTaken => plain(StatusCode::CONFLICT, "path_taken"),
        FilesError::Empty => plain(StatusCode::BAD_REQUEST, "empty"),
        FilesError::TooLarge { limit } => (
            StatusCode::PAYLOAD_TOO_LARGE,
            json!({ "error": "too_large", "limit": limit }),
        ),
        FilesError::UnsupportedMime => {
            plain(StatusCode::UNSUPPORTED_MEDIA_TYPE, "unsupported_type")
        }
        FilesError::InvalidText => plain(StatusCode::UNPROCESSABLE_ENTITY, "invalid_text"),
        FilesError::Credential(pattern) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            json!({ "error": "credential", "pattern": pattern.name() }),
        ),
        FilesError::PageOutOfRange { .. } => plain(StatusCode::NOT_FOUND, "page_not_found"),
        FilesError::ConnectionRequired => plain(StatusCode::BAD_REQUEST, "connection_required"),
        FilesError::LinkInvalid => plain(StatusCode::NOT_FOUND, "link_invalid"),
        FilesError::Pdf(_) => plain(StatusCode::UNPROCESSABLE_ENTITY, "unreadable_pdf"),
        FilesError::StoredRowInvalid | FilesError::Storage(_) | FilesError::Database(_) => {
            internal()
        }
    }
}

fn describe_search(error: &SearchError) -> (StatusCode, Value) {
    match error {
        SearchError::Where(_) => plain(StatusCode::BAD_REQUEST, "invalid_where"),
        SearchError::Folder(_) => (
            StatusCode::BAD_REQUEST,
            json!({ "error": "invalid_input", "field": "folder" }),
        ),
        SearchError::Database(_) => internal(),
    }
}

fn describe_transfer(error: &TransferError) -> (StatusCode, Value) {
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
