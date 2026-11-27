use std::sync::Arc;
use std::time::Duration;

use futures_util::{Stream, StreamExt};
use object_store::memory::InMemory;
use openmemory_ops::VersionOp;
use sqlx::PgPool;
use tokio_util::sync::CancellationToken;

use super::*;
use crate::access::Scope;
use crate::files::{FileContent, FileRef, Files};
use crate::journal::cursor::Epoch;
use crate::pages::ids::{PageId, Seq, VersionNumber};
use crate::pages::relocate::move_page;
use crate::purge::Subject;
use crate::test_support::{agent, create, path, scoped};

const WAIT: Duration = Duration::from_secs(5);

struct Running {
    journal: Journal,
    stop: CancellationToken,
}

impl Running {
    fn start(pool: &PgPool) -> Self {
        let journal = Journal::new(pool.clone());
        let stop = CancellationToken::new();
        tokio::spawn(journal.clone().listen(stop.clone()));
        Self { journal, stop }
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        self.stop.cancel();
        self.journal.close();
    }
}

async fn next(events: &mut (impl Stream<Item = Event> + Unpin)) -> Event {
    tokio::time::timeout(WAIT, events.next())
        .await
        .expect("an event arrives")
        .expect("the stream is open")
}

async fn ready(events: &mut (impl Stream<Item = Event> + Unpin)) -> (Cursor, bool) {
    match next(events).await {
        Event::Ready { cursor, resumed } => (cursor, resumed),
        other => panic!("expected ready, got {other:?}"),
    }
}

async fn change(events: &mut (impl Stream<Item = Event> + Unpin)) -> Change {
    match next(events).await {
        Event::Change { change, .. } => change,
        other => panic!("expected a change, got {other:?}"),
    }
}

fn page_change(id: i64, version: i32, op: VersionOp, page_path: &str) -> Change {
    Change::Page {
        id: PageId(id),
        version: VersionNumber(version),
        op,
        path: Some(path(page_path)),
        previous_path: None,
    }
}

async fn page_id(pool: &PgPool, page_path: &str) -> i64 {
    sqlx::query_scalar!("SELECT id FROM pages WHERE path = $1", page_path)
        .fetch_one(pool)
        .await
        .unwrap()
}

#[test]
fn a_cursor_reads_back_what_it_writes() {
    let cursor = Cursor {
        epoch: Epoch {
            system: -7_712_044_897_833_390_116,
            timeline: 3,
        },
        seq: Seq(42),
    };
    assert_eq!(cursor.to_string().parse::<Cursor>(), Ok(cursor));
    assert!("1.2".parse::<Cursor>().is_err());
    assert!("1.2.3.4".parse::<Cursor>().is_err());
    assert!("a.2.3".parse::<Cursor>().is_err());
}

#[sqlx::test]
async fn a_commit_never_lands_behind_a_sequence_already_read(pool: PgPool) {
    let actor = agent(&pool, "mino", "claude").await;
    create(&pool, &actor, "first", "one\n").await;
    let mut slow = pool.begin().await.unwrap();
    sqlx::query!(
        "INSERT INTO purges (kind, subject_id, path, user_id) VALUES ('page', 999, 'gone', $1)",
        actor.user_id.0
    )
    .execute(&mut *slow)
    .await
    .unwrap();
    create(&pool, &actor, "second", "two\n").await;
    let read = sqlx::query_scalar!(r#"SELECT max(seq) AS "seq!" FROM journal"#)
        .fetch_one(&pool)
        .await
        .unwrap();
    slow.commit().await.unwrap();
    let purge_seq = sqlx::query_scalar!("SELECT seq FROM journal WHERE purge_id IS NOT NULL")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(purge_seq > read, "{purge_seq} lands after {read}");
}

#[sqlx::test]
async fn a_fresh_stream_starts_at_the_head_and_then_follows_commits(pool: PgPool) {
    let running = Running::start(&pool);
    let actor = agent(&pool, "mino", "claude").await;
    create(&pool, &actor, "before", "old\n").await;
    let mut events = Box::pin(running.journal.follow(Scope::everything(), None));
    let (_, resumed) = ready(&mut events).await;
    assert!(!resumed);

    create(&pool, &actor, "notes/a", "hello\n").await;
    let id = page_id(&pool, "notes/a").await;
    assert_eq!(
        change(&mut events).await,
        page_change(id, 1, VersionOp::Create, "notes/a")
    );

    move_page(&pool, &actor, &path("notes/a"), &path("notes/b"), None)
        .await
        .unwrap();
    assert_eq!(
        change(&mut events).await,
        Change::Page {
            id: PageId(id),
            version: VersionNumber(2),
            op: VersionOp::Move,
            path: Some(path("notes/b")),
            previous_path: Some(path("notes/a")),
        }
    );

    crate::pages::purge::purge(&pool, actor.user_id, PageId(id))
        .await
        .unwrap();
    assert_eq!(
        change(&mut events).await,
        Change::Purge {
            subject: Subject::Page,
            id,
            path: path("notes/b"),
        }
    );
}

#[sqlx::test]
async fn file_uploads_and_deletes_are_followed(pool: PgPool) {
    let running = Running::start(&pool);
    let actor = agent(&pool, "mino", "claude").await;
    let files = Files::new(pool.clone(), Arc::new(InMemory::new()), 1024);
    let mut events = Box::pin(running.journal.follow(Scope::everything(), None));
    ready(&mut events).await;

    let record = files
        .upload(
            &actor,
            "notes/a.txt",
            FileContent::Text("hello".to_owned()),
            "text/plain",
        )
        .await
        .unwrap();
    files.delete(&actor, &FileRef::Id(record.id)).await.unwrap();

    for op in [FileOp::Upload, FileOp::Delete] {
        assert_eq!(
            change(&mut events).await,
            Change::File {
                id: record.id,
                op,
                path: path("notes/a.txt"),
            }
        );
    }
}

#[sqlx::test]
async fn a_cursor_resumes_right_after_its_event(pool: PgPool) {
    let running = Running::start(&pool);
    let actor = agent(&pool, "mino", "claude").await;
    let mut first = Box::pin(running.journal.follow(Scope::everything(), None));
    ready(&mut first).await;
    create(&pool, &actor, "a", "a\n").await;
    let Event::Change { cursor, .. } = next(&mut first).await else {
        panic!("expected a change");
    };
    create(&pool, &actor, "b", "b\n").await;

    let mut resumed = Box::pin(running.journal.follow(Scope::everything(), Some(cursor)));
    assert_eq!(ready(&mut resumed).await, (cursor, true));
    let id = page_id(&pool, "b").await;
    assert_eq!(
        change(&mut resumed).await,
        page_change(id, 1, VersionOp::Create, "b")
    );
}

#[sqlx::test]
async fn a_cursor_from_another_history_starts_over(pool: PgPool) {
    let running = Running::start(&pool);
    let actor = agent(&pool, "mino", "claude").await;
    create(&pool, &actor, "a", "a\n").await;
    let mut events = Box::pin(running.journal.follow(Scope::everything(), None));
    let (head, _) = ready(&mut events).await;

    let restored = Cursor {
        epoch: Epoch {
            timeline: head.epoch.timeline + 1,
            ..head.epoch
        },
        seq: Seq(0),
    };
    let ahead = head.at(Seq(head.seq.0 + 10));
    for stale in [restored, ahead] {
        let mut events = Box::pin(running.journal.follow(Scope::everything(), Some(stale)));
        assert_eq!(ready(&mut events).await, (head, false));
    }
}

#[sqlx::test]
async fn a_scoped_reader_sees_only_paths_inside_its_folders(pool: PgPool) {
    let running = Running::start(&pool);
    let actor = agent(&pool, "mino", "claude").await;
    let reader = scoped(&actor, &["clients"], false);
    create(&pool, &actor, "clients/acme", "acme\n").await;
    let id = page_id(&pool, "clients/acme").await;
    let mut events = Box::pin(running.journal.follow(reader.scope.clone(), None));
    ready(&mut events).await;

    move_page(
        &pool,
        &actor,
        &path("clients/acme"),
        &path("private/acme"),
        None,
    )
    .await
    .unwrap();
    assert_eq!(
        change(&mut events).await,
        Change::Page {
            id: PageId(id),
            version: VersionNumber(2),
            op: VersionOp::Move,
            path: None,
            previous_path: Some(path("clients/acme")),
        }
    );

    create(&pool, &actor, "private/diary", "secret\n").await;
    let Event::Passed { cursor } = next(&mut events).await else {
        panic!("a hidden change only moves the cursor");
    };
    let mut resumed = Box::pin(running.journal.follow(reader.scope, Some(cursor)));
    assert_eq!(ready(&mut resumed).await, (cursor, true));
}

#[sqlx::test]
async fn closing_the_journal_ends_every_stream(pool: PgPool) {
    let running = Running::start(&pool);
    let mut events = Box::pin(running.journal.follow(Scope::everything(), None));
    ready(&mut events).await;
    running.journal.close();
    let ended = tokio::time::timeout(WAIT, events.next()).await.unwrap();
    assert_eq!(ended, None);
}
