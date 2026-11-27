use sqlx::PgConnection;

use crate::files::FileId;
use crate::journal::cursor::{Epoch, Position};
use crate::pages::ids::{PageId, Seq, VersionNumber};

#[derive(Debug)]
pub(super) struct EntryRow {
    pub(super) seq: Seq,
    pub(super) page_id: Option<PageId>,
    pub(super) n: Option<VersionNumber>,
    pub(super) page_path: Option<String>,
    pub(super) page_op: Option<String>,
    pub(super) previous_path: Option<String>,
    pub(super) file_id: Option<FileId>,
    pub(super) file_op: Option<String>,
    pub(super) file_path: Option<String>,
    pub(super) purge_kind: Option<String>,
    pub(super) purge_subject_id: Option<i64>,
    pub(super) purge_path: Option<String>,
}

pub(super) async fn position(conn: &mut PgConnection) -> Result<Position, sqlx::Error> {
    // A WAL file name starts with the current timeline, and every restore starts a new one.
    let row = sqlx::query!(
        r#"SELECT
  s.system_identifier AS "system!",
  ('x' || left(pg_walfile_name(pg_current_wal_lsn()), 8))::bit(32)::integer AS "timeline!",
  (SELECT COALESCE(max(seq), 0) FROM journal) AS "head!: Seq"
FROM pg_control_system() AS s"#
    )
    .fetch_one(conn)
    .await?;
    Ok(Position {
        epoch: Epoch {
            system: row.system,
            timeline: row.timeline,
        },
        head: row.head,
    })
}

pub(super) async fn after(
    conn: &mut PgConnection,
    seq: Seq,
    limit: i64,
) -> Result<Vec<EntryRow>, sqlx::Error> {
    sqlx::query_as!(
        EntryRow,
        r#"SELECT
  j.seq AS "seq!: Seq",
  j.page_id AS "page_id?: PageId",
  j.n AS "n?: VersionNumber",
  v.path AS "page_path?",
  v.op AS "page_op?",
  previous.path AS "previous_path?",
  j.file_id AS "file_id?: FileId",
  j.file_op AS "file_op?",
  f.path AS "file_path?",
  p.kind AS "purge_kind?",
  p.subject_id AS "purge_subject_id?",
  p.path AS "purge_path?"
FROM journal AS j
LEFT JOIN page_versions AS v ON v.page_id = j.page_id AND v.n = j.n
LEFT JOIN page_versions AS previous ON previous.page_id = j.page_id AND previous.n = j.n - 1
LEFT JOIN files AS f ON f.id = j.file_id
LEFT JOIN purges AS p ON p.id = j.purge_id
WHERE j.seq > $1
ORDER BY j.seq
LIMIT $2"#,
        seq as Seq,
        limit
    )
    .fetch_all(conn)
    .await
}
