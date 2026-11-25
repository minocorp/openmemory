mod assessment;
mod config;
mod status_file;

use std::path::Path;
use std::sync::Arc;

use sqlx::PgPool;
use time::{Duration, OffsetDateTime};

pub(crate) use assessment::{BackupProblem, BackupReport};
pub(crate) use config::BackupConfig;

#[derive(Debug, Clone, Default)]
pub(crate) struct Backup {
    status_file: Option<Arc<Path>>,
}

impl Backup {
    pub(crate) fn from_config(config: &BackupConfig) -> Self {
        Self {
            status_file: config.status_file().map(Arc::from),
        }
    }

    #[cfg(test)]
    pub(crate) fn watching(path: &Path) -> Self {
        Self {
            status_file: Some(Arc::from(path)),
        }
    }

    pub(crate) async fn report(&self, pool: &PgPool) -> Option<BackupReport> {
        let path = self.status_file.as_deref()?;
        let wal_age = wal_archive_age(pool).await;
        let now = OffsetDateTime::now_utc();
        match status_file::read(path).await {
            Ok(file) => Some(assessment::assess(&file, wal_age, now)),
            Err(error) => {
                tracing::warn!(%error, "backup status unavailable");
                Some(BackupReport::without_status(wal_age))
            }
        }
    }
}

async fn wal_archive_age(pool: &PgPool) -> Option<Duration> {
    let seconds = sqlx::query_scalar!(
        r#"SELECT EXTRACT(EPOCH FROM now() - last_archived_time)::float8 AS "seconds?"
FROM pg_stat_archiver"#
    )
    .fetch_one(pool)
    .await
    .inspect_err(|error| tracing::warn!(%error, "read wal archive state failed"))
    .ok()??;
    Some(Duration::seconds_f64(seconds))
}

#[cfg(test)]
mod tests;
