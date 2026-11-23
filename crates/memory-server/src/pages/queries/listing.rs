use sqlx::PgConnection;
use time::OffsetDateTime;

use crate::pages::guard::START_PATH;
use crate::pages::ids::{PageId, VersionNumber};
use crate::pages::scope_filter::ScopeFilter;

#[derive(Debug)]
pub(crate) struct ListedPage {
    pub(crate) id: PageId,
    pub(crate) path: String,
    pub(crate) current_version: VersionNumber,
    pub(crate) updated_at: OffsetDateTime,
}

#[derive(Debug)]
pub(crate) struct ExportedPage {
    pub(crate) path: String,
    pub(crate) content: String,
}

pub(crate) async fn list_live(
    conn: &mut PgConnection,
    scope: &ScopeFilter,
) -> Result<Vec<ListedPage>, sqlx::Error> {
    sqlx::query_as!(
        ListedPage,
        r#"SELECT
  p.id AS "id: PageId",
  p.path,
  p.current_version AS "current_version: VersionNumber",
  v.created_at AS updated_at
FROM pages AS p
JOIN page_versions AS v ON v.page_id = p.id AND v.n = p.current_version
WHERE p.deleted_at IS NULL
  AND ($1 OR p.path = $2 OR p.path = ANY($3) OR p.path LIKE ANY($4))
ORDER BY p.path"#,
        scope.unrestricted,
        START_PATH,
        &scope.exact,
        &scope.patterns
    )
    .fetch_all(conn)
    .await
}

pub(crate) async fn list_live_content(
    conn: &mut PgConnection,
    scope: &ScopeFilter,
) -> Result<Vec<ExportedPage>, sqlx::Error> {
    sqlx::query_as!(
        ExportedPage,
        r#"SELECT
  path,
  content
FROM pages
WHERE deleted_at IS NULL
  AND ($1 OR path = ANY($2) OR path LIKE ANY($3))
ORDER BY path"#,
        scope.unrestricted,
        &scope.exact,
        &scope.patterns
    )
    .fetch_all(conn)
    .await
}

pub(crate) async fn ids_for_paths(
    conn: &mut PgConnection,
    paths: &[String],
) -> Result<Vec<(String, PageId)>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"SELECT
  path,
  id AS "id: PageId"
FROM pages
WHERE path = ANY($1)
  AND deleted_at IS NULL"#,
        paths
    )
    .fetch_all(conn)
    .await?;
    Ok(rows.into_iter().map(|row| (row.path, row.id)).collect())
}
