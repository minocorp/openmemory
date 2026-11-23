use sqlx::PgConnection;
use time::OffsetDateTime;

use crate::pages::ids::{GroupId, VersionNumber};
use crate::pages::scope_filter::ScopeFilter;

#[derive(Debug)]
pub(crate) struct HistoryRow {
    pub(crate) n: VersionNumber,
    pub(crate) path: String,
    pub(crate) op: String,
    pub(crate) reverts_version: Option<VersionNumber>,
    pub(crate) author_name: String,
    pub(crate) group_id: GroupId,
    pub(crate) created_at: OffsetDateTime,
    pub(crate) content: String,
    pub(crate) previous_content: Option<String>,
}

pub(crate) struct HistoryFilter<'a> {
    pub(crate) page_id: Option<i64>,
    pub(crate) by: Option<&'a str>,
    pub(crate) since: Option<OffsetDateTime>,
    pub(crate) scope: &'a ScopeFilter,
    pub(crate) limit: i64,
    pub(crate) before_version: Option<i32>,
}

pub(crate) async fn list(
    conn: &mut PgConnection,
    filter: &HistoryFilter<'_>,
) -> Result<Vec<HistoryRow>, sqlx::Error> {
    sqlx::query_as!(
        HistoryRow,
        r#"SELECT
  v.n AS "n: VersionNumber",
  v.path,
  v.op,
  v.reverts_version AS "reverts_version: VersionNumber",
  v.author_name,
  v.group_id AS "group_id: GroupId",
  v.created_at,
  v.content,
  previous.content AS "previous_content?"
FROM page_versions AS v
LEFT JOIN page_versions AS previous ON previous.page_id = v.page_id AND previous.n = v.n - 1
WHERE ($1::bigint IS NULL OR v.page_id = $1)
  AND ($2::text IS NULL OR v.author_name = $2)
  AND ($3::timestamptz IS NULL OR v.created_at >= $3)
  AND ($4 OR v.path = ANY($5) OR v.path LIKE ANY($6))
  AND ($8::integer IS NULL OR v.n < $8)
ORDER BY v.seq DESC
LIMIT $7"#,
        filter.page_id,
        filter.by,
        filter.since,
        filter.scope.unrestricted,
        &filter.scope.exact,
        &filter.scope.patterns,
        filter.limit,
        filter.before_version
    )
    .fetch_all(conn)
    .await
}
