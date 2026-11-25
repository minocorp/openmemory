use sqlx::PgConnection;

use crate::pages::scope_filter::ScopeFilter;

pub(crate) struct ChildRow {
    pub(crate) name: String,
    pub(crate) is_page: bool,
    pub(crate) is_folder: bool,
}

pub(crate) async fn children(
    conn: &mut PgConnection,
    prefix: &str,
    scope: &ScopeFilter,
) -> Result<Vec<ChildRow>, sqlx::Error> {
    sqlx::query_as!(
        ChildRow,
        r#"SELECT
  split_part(rest, '/', 1) AS "name!",
  bool_or(position('/' IN rest) = 0) AS "is_page!",
  bool_or(position('/' IN rest) > 0) AS "is_folder!"
FROM (
  SELECT substr(p.path, length($1::text) + 1) AS rest
  FROM pages AS p
  WHERE p.deleted_at IS NULL
    AND starts_with(p.path, $1)
    AND ($2 OR p.path = ANY($3) OR p.path LIKE ANY($4))
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
