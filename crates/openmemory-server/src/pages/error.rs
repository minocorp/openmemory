use openmemory_ops::{BlockedPage, CredentialPattern, PageError, PagePathError, ReplaceError};
use time::Date;

use crate::pages::ids::VersionNumber;

#[derive(Debug, thiserror::Error)]
pub(crate) enum PagesError {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("page not found")]
    NotFound,
    #[error("page was purged")]
    Purged,
    #[error("version not found")]
    VersionNotFound,
    #[error("group not found")]
    GroupNotFound,
    #[error("path is outside the connection scope")]
    OutOfScope,
    #[error("connection cannot write")]
    ReadOnly,
    #[error("path is reserved")]
    ReservedPath,
    #[error("path is already taken")]
    PathTaken,
    #[error("source and target path are the same")]
    SamePath,
    #[error("page changed since the base version")]
    Conflict { current_version: VersionNumber },
    #[error("page is larger than the size limit")]
    TooLarge,
    #[error("text matches a credential pattern")]
    Credential(CredentialPattern),
    #[error("page breaks the grammar")]
    Grammar(#[from] PageError),
    #[error("existing timeline lines cannot change")]
    TimelineChanged,
    #[error("replace failed")]
    Replace(#[from] ReplaceError),
    #[error("timeline date is after today")]
    FutureDate { today: Date },
    #[error("quoted source text could not be verified")]
    QuoteUnverified {
        problem: QuoteProblem,
        path: String,
        page: Option<u32>,
    },
    #[error("write changes nothing")]
    NoChange,
    #[error("page is already in that state")]
    AlreadyCurrent,
    #[error("revert is blocked")]
    RevertBlocked(Vec<BlockedPage>),
    #[error("task label must be 1 to 200 characters")]
    TaskLength,
    #[error("stored path is invalid")]
    StoredPath(#[source] PagePathError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum QuoteProblem {
    NoFile,
    NoPage,
    NoText,
    NotInText,
}

impl PagesError {
    pub(crate) fn is_unique_violation(error: &sqlx::Error) -> bool {
        error
            .as_database_error()
            .is_some_and(sqlx::error::DatabaseError::is_unique_violation)
    }
}
