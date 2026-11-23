use sqlx::PgPool;

use super::*;

fn stamp(at: OffsetDateTime) -> String {
    at.format(&time::format_description::well_known::Rfc3339)
        .unwrap()
}

#[sqlx::test]
async fn without_a_status_file_setting_nothing_is_monitored(pool: PgPool) {
    assert!(Backup::default().report(&pool).await.is_none());
}

#[sqlx::test]
async fn a_missing_status_file_is_degraded(pool: PgPool) {
    let directory = tempfile::tempdir().unwrap();
    let backup = Backup::watching(&directory.path().join("status.json"));
    let report = backup.report(&pool).await.unwrap();
    assert!(report.is_degraded());
}

#[sqlx::test]
async fn a_status_file_is_parsed_and_assessed(pool: PgPool) {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("status.json");
    let now = OffsetDateTime::now_utc();
    let recent = stamp(now - Duration::hours(1));
    let body = format!(
        r#"{{"updated_at":"{}","last_full_at":"{recent}","last_backup_at":"{recent}","off_host":true,
"blobs":{{"in_backup":false,"last_snapshot_at":null}},"restore_test":{{"at":"{recent}","ok":true}}}}"#,
        stamp(now),
    );
    tokio::fs::write(&path, body).await.unwrap();
    let report = Backup::watching(&path).report(&pool).await.unwrap();
    assert_eq!(serde_json::to_value(&report).unwrap()["off_host"], true);
    assert_eq!(report.problems().len(), 1);
}
