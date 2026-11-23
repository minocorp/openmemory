use memory_ops::{CredentialPattern, PagePathError};

use crate::files::pdf::PdfError;

#[derive(Debug, thiserror::Error)]
pub(crate) enum FilesError {
    #[error("invalid path")]
    InvalidPath(#[from] PagePathError),
    #[error("not allowed to write this path")]
    Forbidden,
    #[error("file not found")]
    NotFound,
    #[error("file was purged")]
    Purged,
    #[error("path is taken by a page or file")]
    PathTaken,
    #[error("file is empty")]
    Empty,
    #[error("file exceeds {limit} bytes")]
    TooLarge { limit: u64 },
    #[error("file type is not allowed")]
    UnsupportedMime,
    #[error("text file is not valid utf-8")]
    InvalidText,
    #[error("text contains a credential")]
    Credential(CredentialPattern),
    #[error("page {page} does not exist")]
    PageOutOfRange { page: u32 },
    #[error("an upload link needs a connection")]
    ConnectionRequired,
    #[error("upload link is invalid, expired or used")]
    LinkInvalid,
    #[error("stored file row is invalid")]
    StoredRowInvalid,
    #[error("pdf text extraction failed")]
    Pdf(#[from] PdfError),
    #[error("blob storage failed")]
    Storage(#[from] object_store::Error),
    #[error("database query failed")]
    Database(#[from] sqlx::Error),
}
