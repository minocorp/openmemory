use sqlx::PgConnection;
use time::OffsetDateTime;

use crate::access::UserId;
use crate::pages::ids::{GroupId, PageId, Seq, VersionNumber};
use crate::pages::scope_filter::ScopeFilter;

#[derive(Debug)]
pub(crate) struct ChangeRow {
    pub(crate) group_id: GroupId,
    pub(crate) task: Option<String>,
    pub(crate) page_id: PageId,
    pub(crate) n: VersionNumber,
    pub(crate) seq: Seq,
    pub(crate) path: String,
    pub(crate) op: String,
    pub(crate) author_name: String,
    pub(crate) author_display_name: String,
    pub(crate) created_at: OffsetDateTime,
    pub(crate) content: String,
    pub(crate) previous_content: Option<String>,
}

pub(crate) async fn list_since(
    conn: &mut PgConnection,
    since: Seq,
    scope: &ScopeFilter,
    viewer: UserId,
    limit: i64,
) -> Result<Vec<ChangeRow>, sqlx::Error> {
    sqlx::query_as!(
        ChangeRow,
        r#"SELECT
  g.id AS "group_id: GroupId",
  g.task,
  v.page_id AS "page_id: PageId",
  v.n AS "n: VersionNumber",
  v.seq AS "seq: Seq",
  v.path,
  v.op,
  v.author_name,
  u.display_name AS author_display_name,
  v.created_at,
  v.content,
  previous.content AS "previous_content?"
FROM page_versions AS v
JOIN change_groups AS g ON g.id = v.group_id
JOIN users AS u ON u.id = v.user_id
LEFT JOIN page_versions AS previous ON previous.page_id = v.page_id AND previous.n = v.n - 1
WHERE v.seq > $1
  AND ($2 OR v.path = ANY($3) OR v.path LIKE ANY($4))
  AND (v.user_id <> $6 OR v.connection_id IS NOT NULL)
ORDER BY v.seq
LIMIT $5"#,
        since as Seq,
        scope.unrestricted,
        &scope.exact,
        &scope.patterns,
        limit,
        viewer as UserId
    )
    .fetch_all(conn)
    .await
}

pub(crate) async fn last_seen(
    conn: &mut PgConnection,
    user_id: UserId,
) -> Result<Option<Seq>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT last_seen_seq AS "last_seen_seq: Seq" FROM users WHERE id = $1"#,
        user_id as UserId
    )
    .fetch_optional(conn)
    .await
}

pub(crate) async fn mark_seen(
    conn: &mut PgConnection,
    user_id: UserId,
    seq: Seq,
) -> Result<Option<Seq>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"UPDATE users
SET last_seen_seq = GREATEST(
  last_seen_seq,
  LEAST($2, (SELECT COALESCE(MAX(seq), 0) FROM page_versions))
)
WHERE id = $1
RETURNING last_seen_seq AS "last_seen_seq: Seq""#,
        user_id as UserId,
        seq as Seq
    )
    .fetch_optional(conn)
    .await
}
