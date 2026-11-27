use std::time::Duration;

use sqlx::postgres::PgListener;
use tokio_util::sync::CancellationToken;

use crate::journal::Journal;

const CHANNEL: &str = "journal";
const RETRY_DELAY: Duration = Duration::from_secs(5);

impl Journal {
    pub(crate) async fn listen(self, cancel: CancellationToken) {
        loop {
            if let Err(error) = self.relay(&cancel).await {
                tracing::warn!(%error, "journal listener failed");
            }
            tokio::select! {
                () = cancel.cancelled() => return,
                () = tokio::time::sleep(RETRY_DELAY) => {}
            }
        }
    }

    async fn relay(&self, cancel: &CancellationToken) -> Result<(), sqlx::Error> {
        let mut listener = PgListener::connect_with(&self.pool).await?;
        listener.listen(CHANNEL).await?;
        // Commits made while no listener was connected sent no notification, so readers catch up now.
        self.wake.send_replace(());
        loop {
            tokio::select! {
                () = cancel.cancelled() => return Ok(()),
                received = listener.try_recv() => match received? {
                    Some(_) => {
                        self.wake.send_replace(());
                    }
                    None => return Ok(()),
                },
            }
        }
    }
}
