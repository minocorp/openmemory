use std::collections::VecDeque;

use futures_util::Stream;
use tokio::sync::watch;

use crate::access::Scope;
use crate::journal::cursor::Cursor;
use crate::journal::entry::{Change, Entry};
use crate::journal::{Journal, JournalError, queries};

const BATCH: i64 = 500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Event {
    Ready { cursor: Cursor, resumed: bool },
    Change { cursor: Cursor, change: Change },
    Passed { cursor: Cursor },
}

#[derive(Debug)]
struct Follower {
    journal: Journal,
    scope: Scope,
    from: Option<Cursor>,
    at: Option<Cursor>,
    pending: VecDeque<Event>,
    progress: Progress,
    wake: watch::Receiver<()>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Progress {
    More,
    CaughtUp,
}

impl Journal {
    pub(crate) fn follow(
        &self,
        scope: Scope,
        from: Option<Cursor>,
    ) -> impl Stream<Item = Event> + Send + use<> {
        let follower = Follower {
            journal: self.clone(),
            scope,
            from,
            at: None,
            pending: VecDeque::new(),
            progress: Progress::More,
            wake: self.wake.subscribe(),
        };
        futures_util::stream::unfold(follower, Follower::next)
    }
}

impl Follower {
    async fn next(mut self) -> Option<(Event, Self)> {
        loop {
            if let Some(event) = self.pending.pop_front() {
                return Some((event, self));
            }
            if self.journal.closed.is_cancelled() {
                return None;
            }
            if self.progress == Progress::CaughtUp {
                tokio::select! {
                    changed = self.wake.changed() => changed.ok()?,
                    () = self.journal.closed.cancelled() => return None,
                }
            }
            match self.advance().await {
                Ok(progress) => self.progress = progress,
                Err(error) => {
                    tracing::warn!(%error, "change stream stopped");
                    return None;
                }
            }
        }
    }

    async fn advance(&mut self) -> Result<Progress, JournalError> {
        let mut conn = self.journal.pool.acquire().await?;
        let Some(at) = self.at else {
            let position = queries::position(&mut conn).await?;
            let (at, resumed) = match self.from {
                Some(from) if from.continues(position) => (from, true),
                Some(_) | None => (position.cursor(), false),
            };
            self.at = Some(at);
            self.pending.push_back(Event::Ready {
                cursor: at,
                resumed,
            });
            return Ok(Progress::More);
        };
        self.wake.mark_unchanged();
        let rows = queries::after(&mut conn, at.seq, BATCH).await?;
        let is_full = i64::try_from(rows.len()).is_ok_and(|count| count >= BATCH);
        let mut cursor = at;
        let mut is_hidden_tail = false;
        for row in rows {
            let entry = Entry::try_from(row)?;
            cursor = cursor.at(entry.seq);
            is_hidden_tail = match entry.change.visible_to(&self.scope) {
                Some(change) => {
                    self.pending.push_back(Event::Change { cursor, change });
                    false
                }
                None => true,
            };
        }
        if is_hidden_tail {
            self.pending.push_back(Event::Passed { cursor });
        }
        self.at = Some(cursor);
        Ok(if is_full {
            Progress::More
        } else {
            Progress::CaughtUp
        })
    }
}
