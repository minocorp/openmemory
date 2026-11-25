use sqlx::PgConnection;

use crate::pages::scope_filter::ScopeFilter;

pub(crate) async fn field_names(
    conn: &mut PgConnection,
    scope: &ScopeFilter,
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT DISTINCT f.key AS "key!"
FROM page_fields AS f
JOIN pages AS p ON p.id = f.page_id
WHERE p.deleted_at IS NULL
  AND ($1 OR p.path = ANY($2) OR p.path LIKE ANY($3))
ORDER BY 1"#,
        scope.unrestricted,
        &scope.exact,
        &scope.patterns
    )
    .fetch_all(conn)
    .await
}

pub(crate) async fn hidden_folder_count(
    conn: &mut PgConnection,
    scope: &ScopeFilter,
) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT COUNT(DISTINCT regexp_replace(p.path, '/[^/]*$', '')) AS "count!"
FROM pages AS p
WHERE p.deleted_at IS NULL
  AND p.path LIKE '%/%'
  AND NOT ($1 OR p.path = ANY($2) OR p.path LIKE ANY($3))"#,
        scope.unrestricted,
        &scope.exact,
        &scope.patterns
    )
    .fetch_one(conn)
    .await
}
