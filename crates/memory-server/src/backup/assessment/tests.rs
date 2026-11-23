use time::macros::datetime;

use super::*;
use crate::backup::status_file::Blobs;

const NOW: OffsetDateTime = datetime!(2026-11-23 12:00 UTC);
const FRESH_WAL: Option<Duration> = Some(Duration::minutes(2));

fn healthy() -> StatusFile {
    StatusFile {
        updated_at: NOW - Duration::minutes(1),
        last_full_at: Some(NOW - Duration::days(2)),
        last_backup_at: Some(NOW - Duration::hours(5)),
        off_host: true,
        blobs: Blobs {
            in_backup: true,
            last_snapshot_at: Some(NOW - Duration::minutes(30)),
        },
        restore_test: Some(RestoreTest {
            at: NOW - Duration::days(3),
            ok: true,
        }),
    }
}

fn problems(file: &StatusFile, wal_age: Option<Duration>) -> Vec<BackupProblem> {
    assess(file, wal_age, NOW).problems
}

#[test]
fn recent_backups_archive_and_restore_test_are_healthy() {
    let report = assess(&healthy(), FRESH_WAL, NOW);
    assert!(!report.is_degraded());
    assert_eq!(report.last_full_age_seconds, Some(2 * 86_400));
    assert!(report.off_host);
}

#[test]
fn an_old_full_backup_is_reported() {
    let mut file = healthy();
    file.last_full_at = Some(NOW - Duration::days(9));
    assert_eq!(problems(&file, FRESH_WAL), [BackupProblem::FullBackupStale]);
}

#[test]
fn a_missing_backup_is_stale() {
    let mut file = healthy();
    file.last_full_at = None;
    file.last_backup_at = None;
    assert_eq!(
        problems(&file, FRESH_WAL),
        [BackupProblem::FullBackupStale, BackupProblem::BackupStale]
    );
}

#[test]
fn an_old_or_unknown_wal_archive_is_reported() {
    let file = healthy();
    assert_eq!(
        problems(&file, Some(Duration::minutes(20))),
        [BackupProblem::WalArchiveStale]
    );
    assert_eq!(problems(&file, None), [BackupProblem::WalArchiveStale]);
}

#[test]
fn a_status_file_that_stopped_updating_is_reported() {
    let mut file = healthy();
    file.updated_at = NOW - Duration::minutes(30);
    assert_eq!(problems(&file, FRESH_WAL), [BackupProblem::StatusStale]);
}

#[test]
fn blob_backup_age_counts_only_when_blobs_are_in_the_backup() {
    let mut file = healthy();
    file.blobs.last_snapshot_at = Some(NOW - Duration::hours(4));
    assert_eq!(problems(&file, FRESH_WAL), [BackupProblem::BlobBackupStale]);
    file.blobs.in_backup = false;
    assert_eq!(problems(&file, FRESH_WAL), Vec::<BackupProblem>::new());
}

#[test]
fn a_failed_restore_test_is_reported() {
    let mut file = healthy();
    file.restore_test = Some(RestoreTest {
        at: NOW - Duration::hours(1),
        ok: false,
    });
    assert_eq!(
        problems(&file, FRESH_WAL),
        [BackupProblem::RestoreTestFailed]
    );
}

#[test]
fn a_missing_restore_test_is_pending_until_the_grace_period_ends() {
    let mut file = healthy();
    file.restore_test = None;
    file.last_full_at = Some(NOW - Duration::hours(3));
    assert_eq!(problems(&file, FRESH_WAL), Vec::<BackupProblem>::new());
    file.last_full_at = Some(NOW - Duration::days(3));
    assert_eq!(
        problems(&file, FRESH_WAL),
        [BackupProblem::RestoreTestStale]
    );
}

#[test]
fn an_old_restore_test_is_reported() {
    let mut file = healthy();
    file.restore_test = Some(RestoreTest {
        at: NOW - Duration::days(16),
        ok: true,
    });
    assert_eq!(
        problems(&file, FRESH_WAL),
        [BackupProblem::RestoreTestStale]
    );
}
