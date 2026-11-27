use openmemory_ops::{PagePathError, WhereError};

#[derive(Debug, thiserror::Error)]
pub(crate) enum SearchError {
    #[error("database error")]
    Database(#[from] sqlx::Error),
    #[error("where condition is invalid")]
    Where(#[from] WhereError),
    #[error("folder is not a valid path")]
    Folder(#[source] PagePathError),
    #[error("cursor is not valid")]
    Cursor,
}
