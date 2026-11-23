use sqlx::PgConnection;

use crate::pages::scope_filter::ScopeFilter;

pub(super) struct ChildRow {
    pub(super) name: String,
    pub(super) is_file: bool,
}

pub(super) async fn children(
    conn: &mut PgConnection,
    prefix: &str,
    scope: &ScopeFilter,
) -> Result<Vec<ChildRow>, sqlx::Error> {
    sqlx::query_as!(
        ChildRow,
        r#"SELECT
  split_part(rest, '/', 1) AS "name!",
  bool_or(position('/' IN rest) = 0) AS "is_file!"
FROM (
  SELECT substr(f.path, length($1::text) + 1) AS rest
  FROM files AS f
  WHERE f.deleted_at IS NULL
    AND starts_with(f.path, $1)
    AND ($2 OR f.path = ANY($3) OR f.path LIKE ANY($4))
) AS children
GROUP BY 1
ORDER BY 1"#,
        prefix,
        scope.unrestricted,
        &scope.exact,
        &scope.patterns
    )
    .fetch_all(conn)
    .await
}
