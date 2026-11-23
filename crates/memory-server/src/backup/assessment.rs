use serde::Serialize;
use time::{Duration, OffsetDateTime};
use utoipa::ToSchema;

use super::status_file::{RestoreTest, StatusFile};

const STATUS_MAX_AGE: Duration = Duration::minutes(10);
const FULL_MAX_AGE: Duration = Duration::days(8);
const BACKUP_MAX_AGE: Duration = Duration::hours(36);
const WAL_MAX_AGE: Duration = Duration::minutes(15);
const BLOBS_MAX_AGE: Duration = Duration::hours(3);
const RESTORE_TEST_MAX_AGE: Duration = Duration::days(15);
const RESTORE_TEST_GRACE: Duration = Duration::days(2);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Problem {
    StatusMissing,
    StatusStale,
    FullBackupStale,
    BackupStale,
    WalArchiveStale,
    BlobBackupStale,
    RestoreTestFailed,
    RestoreTestStale,
}

#[derive(Debug, Serialize, ToSchema)]
struct RestoreTestReport {
    #[serde(with = "time::serde::rfc3339")]
    at: OffsetDateTime,
    ok: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub(crate) struct BackupReport {
    problems: Vec<Problem>,
    last_full_age_seconds: Option<i64>,
    last_backup_age_seconds: Option<i64>,
    wal_archive_age_seconds: Option<i64>,
    blobs_age_seconds: Option<i64>,
    off_host: bool,
    restore_test: Option<RestoreTestReport>,
}

impl BackupReport {
    pub(crate) fn problems(&self) -> &[Problem] {
        &self.problems
    }

    pub(crate) fn is_degraded(&self) -> bool {
        !self.problems.is_empty()
    }

    pub(super) fn without_status(wal_age: Option<Duration>) -> Self {
        Self {
            problems: vec![Problem::StatusMissing],
            last_full_age_seconds: None,
            last_backup_age_seconds: None,
            wal_archive_age_seconds: wal_age.map(Duration::whole_seconds),
            blobs_age_seconds: None,
            off_host: false,
            restore_test: None,
        }
    }
}

pub(super) fn assess(
    file: &StatusFile,
    wal_age: Option<Duration>,
    now: OffsetDateTime,
) -> BackupReport {
    let age = |at: Option<OffsetDateTime>| at.map(|at| now - at);
    let full = age(file.last_full_at);
    let backup = age(file.last_backup_at);
    let blobs = file
        .blobs
        .in_backup
        .then(|| age(file.blobs.last_snapshot_at));
    let mut problems = Vec::new();
    if now - file.updated_at > STATUS_MAX_AGE {
        problems.push(Problem::StatusStale);
    }
    if is_stale(full, FULL_MAX_AGE) {
        problems.push(Problem::FullBackupStale);
    }
    if is_stale(backup, BACKUP_MAX_AGE) {
        problems.push(Problem::BackupStale);
    }
    if is_stale(wal_age, WAL_MAX_AGE) {
        problems.push(Problem::WalArchiveStale);
    }
    if blobs.is_some_and(|blobs| is_stale(blobs, BLOBS_MAX_AGE)) {
        problems.push(Problem::BlobBackupStale);
    }
    problems.extend(restore_test_problem(file, full, now));
    BackupReport {
        problems,
        last_full_age_seconds: full.map(Duration::whole_seconds),
        last_backup_age_seconds: backup.map(Duration::whole_seconds),
        wal_archive_age_seconds: wal_age.map(Duration::whole_seconds),
        blobs_age_seconds: blobs.flatten().map(Duration::whole_seconds),
        off_host: file.off_host,
        restore_test: file
            .restore_test
            .map(|RestoreTest { at, ok }| RestoreTestReport { at, ok }),
    }
}

fn is_stale(age: Option<Duration>, limit: Duration) -> bool {
    age.is_none_or(|age| age > limit)
}

fn restore_test_problem(
    file: &StatusFile,
    full_age: Option<Duration>,
    now: OffsetDateTime,
) -> Option<Problem> {
    match file.restore_test {
        Some(test) if !test.ok => Some(Problem::RestoreTestFailed),
        Some(test) if now - test.at > RESTORE_TEST_MAX_AGE => Some(Problem::RestoreTestStale),
        Some(_) => None,
        None => full_age
            .filter(|age| *age > RESTORE_TEST_GRACE)
            .map(|_| Problem::RestoreTestStale),
    }
}

#[cfg(test)]
mod tests;
