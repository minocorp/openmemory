use std::path::Path;

use serde::Deserialize;
use time::OffsetDateTime;

#[derive(Debug, Deserialize)]
pub(super) struct StatusFile {
    #[serde(with = "time::serde::rfc3339")]
    pub(super) updated_at: OffsetDateTime,
    #[serde(with = "time::serde::rfc3339::option")]
    pub(super) last_full_at: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    pub(super) last_backup_at: Option<OffsetDateTime>,
    pub(super) off_host: bool,
    pub(super) blobs: Blobs,
    pub(super) restore_test: Option<RestoreTest>,
}

#[derive(Debug, Deserialize)]
pub(super) struct Blobs {
    pub(super) in_backup: bool,
    #[serde(with = "time::serde::rfc3339::option")]
    pub(super) last_snapshot_at: Option<OffsetDateTime>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub(super) struct RestoreTest {
    #[serde(with = "time::serde::rfc3339")]
    pub(super) at: OffsetDateTime,
    pub(super) ok: bool,
}

#[derive(Debug, thiserror::Error)]
pub(super) enum StatusFileError {
    #[error("read status file")]
    Read(#[source] std::io::Error),
    #[error("parse status file")]
    Parse(#[source] serde_json::Error),
}

pub(super) async fn read(path: &Path) -> Result<StatusFile, StatusFileError> {
    let bytes = tokio::fs::read(path).await.map_err(StatusFileError::Read)?;
    serde_json::from_slice(&bytes).map_err(StatusFileError::Parse)
}
