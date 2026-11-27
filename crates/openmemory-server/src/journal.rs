mod cursor;
mod entry;
mod follow;
mod listen;
mod queries;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use sqlx::PgPool;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

pub(crate) use cursor::Cursor;
pub(crate) use entry::{Change, FileOp};
pub(crate) use follow::Event;

#[derive(Debug, Clone)]
pub(crate) struct Journal {
    pool: PgPool,
    wake: Arc<watch::Sender<()>>,
    closed: CancellationToken,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum JournalError {
    #[error("database")]
    Database(#[from] sqlx::Error),
    #[error("stored journal row is invalid")]
    StoredRowInvalid,
}

impl Journal {
    pub(crate) fn new(pool: PgPool) -> Self {
        let (wake, _) = watch::channel(());
        Self {
            pool,
            wake: Arc::new(wake),
            closed: CancellationToken::new(),
        }
    }

    pub(crate) fn close(&self) {
        self.closed.cancel();
    }
}
