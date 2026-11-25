use bytes::Bytes;
use futures_util::stream::BoxStream;
use openmemory_ops::PagePath;
use time::OffsetDateTime;

use crate::access::UserId;
use crate::files::FileId;
use crate::storage::Sha256Hash;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FileRecord {
    pub(crate) id: FileId,
    pub(crate) path: PagePath,
    pub(crate) sha256: Sha256Hash,
    pub(crate) mime: String,
    pub(crate) size: i64,
    pub(crate) uploader: UserId,
    pub(crate) created_at: OffsetDateTime,
    pub(crate) page_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FileRef {
    Id(FileId),
    Path(PagePath),
}

#[derive(Debug, Clone)]
pub(crate) enum FileContent {
    Bytes(Bytes),
    Text(String),
}

pub(crate) struct FileDownload {
    pub(crate) record: FileRecord,
    pub(crate) stream: BoxStream<'static, Result<Bytes, object_store::Error>>,
}

impl FileContent {
    pub(crate) fn into_bytes(self) -> Bytes {
        match self {
            Self::Bytes(bytes) => bytes,
            Self::Text(text) => Bytes::from(text),
        }
    }
}
