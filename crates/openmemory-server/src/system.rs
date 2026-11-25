use serde::Serialize;
use sqlx::PgPool;
use utoipa::ToSchema;

use crate::backup::{Backup, BackupReport};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub(crate) enum DatabaseStatus {
    Ok,
    Unavailable,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct SystemStatus {
    pub(crate) database: DatabaseStatus,
    pub(crate) backup: Option<BackupReport>,
}

pub(crate) async fn status(pool: &PgPool, backup: &Backup) -> SystemStatus {
    let database = database_status(pool).await;
    let backup = match database {
        DatabaseStatus::Ok => backup.report(pool).await,
        DatabaseStatus::Unavailable => None,
    };
    SystemStatus { database, backup }
}

async fn database_status(pool: &PgPool) -> DatabaseStatus {
    match sqlx::query_scalar!(r#"SELECT 1 AS "ok!""#)
        .fetch_one(pool)
        .await
    {
        Ok(_) => DatabaseStatus::Ok,
        Err(error) => {
            tracing::warn!(%error, "database status check failed");
            DatabaseStatus::Unavailable
        }
    }
}

#[cfg(test)]
mod tests {
    use sqlx::PgPool;

    use super::*;

    #[sqlx::test]
    async fn status_carries_the_backup_service_report(pool: PgPool) {
        let directory = tempfile::tempdir().unwrap();
        let backup = Backup::watching(&directory.path().join("status.json"));
        let status = status(&pool, &backup).await;
        assert_eq!(status.database, DatabaseStatus::Ok);
        assert!(status.backup.is_some_and(|report| report.is_degraded()));
    }

    #[sqlx::test]
    async fn status_has_no_backup_report_when_backups_are_not_monitored(pool: PgPool) {
        let status = status(&pool, &Backup::default()).await;
        assert!(status.backup.is_none());
    }
}
