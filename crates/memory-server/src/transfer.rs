mod export;
mod import;
mod layout;

#[cfg(test)]
mod tests;

use crate::files::FilesError;
use crate::pages::error::PagesError;

pub(crate) use export::export;
pub(crate) use import::{ImportReport, SkipReason, Skipped, import};

pub(crate) const MAX_ARCHIVE_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Debug, thiserror::Error)]
pub(crate) enum TransferError {
    #[error("archive is not a readable zip")]
    InvalidArchive(#[source] zip::result::ZipError),
    #[error("archive is larger than the size limit")]
    TooLarge,
    #[error("archive could not be written")]
    Write(#[source] std::io::Error),
    #[error("page service failed")]
    Pages(#[from] PagesError),
    #[error("file service failed")]
    Files(#[from] FilesError),
}
