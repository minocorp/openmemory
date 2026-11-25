use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use sqlx::PgPool;
use tokio_util::sync::CancellationToken;

use super::run_every;
use crate::backup::Backup;

const INTERVAL: Duration = Duration::from_mins(5);

pub(super) async fn run(pool: PgPool, backup: Backup, cancel: CancellationToken) {
    let was_degraded = AtomicBool::new(false);
    run_every(INTERVAL, &cancel, || async {
        let Some(report) = backup.report(&pool).await else {
            return;
        };
        if report.is_degraded() {
            tracing::warn!(problems = ?report.problems(), "backups are degraded");
        } else if was_degraded.load(Ordering::Relaxed) {
            tracing::info!("backups are healthy again");
        }
        was_degraded.store(report.is_degraded(), Ordering::Relaxed);
    })
    .await;
}
