mod backup_monitor;
mod blob_gc;

use std::future::Future;
use std::sync::Arc;
use std::time::Duration;

use object_store::ObjectStore;
use sqlx::PgPool;
use time::OffsetDateTime;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;

use crate::backup::Backup;
use crate::journal::Journal;
use crate::oauth;
use crate::rate_limit::RateLimits;

const BLOB_GC_INTERVAL: Duration = Duration::from_hours(1);
const PENDING_EXPIRY_INTERVAL: Duration = Duration::from_mins(5);
const RATE_LIMIT_PRUNE_INTERVAL: Duration = Duration::from_mins(1);
const BLOB_GC_DELAY: time::Duration = time::Duration::hours(24);

#[derive(Debug)]
pub(crate) struct Jobs {
    tracker: TaskTracker,
    cancel: CancellationToken,
}

impl Jobs {
    pub(crate) fn start(
        pool: PgPool,
        store: Arc<dyn ObjectStore>,
        rate_limits: RateLimits,
        backup: Backup,
        journal: Journal,
    ) -> Self {
        let tracker = TaskTracker::new();
        let cancel = CancellationToken::new();
        tracker.spawn(journal.listen(cancel.clone()));
        tracker.spawn(run_blob_gc(pool.clone(), store, cancel.clone()));
        tracker.spawn(backup_monitor::run(pool.clone(), backup, cancel.clone()));
        tracker.spawn(run_pending_expiry(pool, cancel.clone()));
        tracker.spawn(run_rate_limit_pruning(rate_limits, cancel.clone()));
        tracker.close();
        Self { tracker, cancel }
    }

    pub(crate) async fn shutdown(self) {
        self.cancel.cancel();
        self.tracker.wait().await;
    }
}

async fn run_blob_gc(pool: PgPool, store: Arc<dyn ObjectStore>, cancel: CancellationToken) {
    run_every(BLOB_GC_INTERVAL, &cancel, || async {
        let older_than = OffsetDateTime::now_utc() - BLOB_GC_DELAY;
        match blob_gc::collect(&pool, &*store, older_than).await {
            Ok(removed) => tracing::info!(removed, "blob garbage collection finished"),
            Err(error) => tracing::warn!(%error, "blob garbage collection failed"),
        }
    })
    .await;
}

async fn run_pending_expiry(pool: PgPool, cancel: CancellationToken) {
    run_every(PENDING_EXPIRY_INTERVAL, &cancel, || async {
        match oauth::expire_pending(&pool).await {
            Ok(removed) if removed > 0 => {
                tracing::info!(removed, "abandoned connection approvals removed");
            }
            Ok(_) => {}
            Err(error) => tracing::warn!(%error, "pending approval expiry failed"),
        }
    })
    .await;
}

async fn run_rate_limit_pruning(rate_limits: RateLimits, cancel: CancellationToken) {
    run_every(RATE_LIMIT_PRUNE_INTERVAL, &cancel, || async {
        rate_limits.prune();
    })
    .await;
}

pub(super) async fn run_every<F, Fut>(period: Duration, cancel: &CancellationToken, mut task: F)
where
    F: FnMut() -> Fut,
    Fut: Future<Output = ()>,
{
    let mut interval = tokio::time::interval(period);
    loop {
        tokio::select! {
            () = cancel.cancelled() => return,
            _ = interval.tick() => {}
        }
        tokio::select! {
            () = cancel.cancelled() => return,
            () = task() => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use object_store::memory::InMemory;

    use super::*;

    #[sqlx::test]
    async fn shutdown_stops_the_background_tasks(pool: PgPool) {
        let jobs = Jobs::start(
            pool.clone(),
            Arc::new(InMemory::new()),
            RateLimits::new(),
            Backup::default(),
            Journal::new(pool),
        );
        tokio::time::timeout(Duration::from_secs(5), jobs.shutdown())
            .await
            .unwrap();
    }
}
