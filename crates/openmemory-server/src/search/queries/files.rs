use sqlx::PgConnection;
use time::OffsetDateTime;

use crate::files::FileId;
use crate::pages::scope_filter::ScopeFilter;

#[derive(Debug)]
pub(crate) struct FileMatchRow {
    pub(crate) id: FileId,
    pub(crate) path: String,
    pub(crate) author: String,
    pub(crate) created_at: OffsetDateTime,
    pub(crate) page: i32,
    pub(crate) text: String,
}

#[derive(Debug)]
pub(crate) struct FilePathRow {
    pub(crate) id: FileId,
    pub(crate) path: String,
    pub(crate) author: String,
    pub(crate) created_at: OffsetDateTime,
}

pub(crate) async fn find(
    conn: &mut PgConnection,
    scope: &ScopeFilter,
    folder: Option<&str>,
    like_pattern: &str,
    limit: i64,
) -> Result<Vec<FileMatchRow>, sqlx::Error> {
    let folder_prefix = folder.map(|folder| format!("{}/%", openmemory_ops::escape_like(folder)));
    sqlx::query_as!(
        FileMatchRow,
        r#"SELECT
  f.id AS "id: FileId",
  f.path,
  u.username AS author,
  f.created_at,
  fp.page,
  fp.text
FROM file_pages AS fp
JOIN files AS f ON f.id = fp.file_id
JOIN users AS u ON u.id = f.user_id
WHERE f.deleted_at IS NULL
  AND fp.text LIKE $1
  AND ($2 OR f.path = ANY($3) OR f.path LIKE ANY($4))
  AND ($5::text IS NULL OR f.path = $5 OR f.path LIKE $6)
ORDER BY f.created_at DESC, f.id DESC, fp.page
LIMIT $7"#,
        like_pattern,
        scope.unrestricted,
        &scope.exact,
        &scope.patterns,
        folder,
        folder_prefix,
        limit
    )
    .fetch_all(conn)
    .await
}

pub(crate) async fn find_by_path(
    conn: &mut PgConnection,
    scope: &ScopeFilter,
    folder: Option<&str>,
    like_pattern: &str,
    limit: i64,
) -> Result<Vec<FilePathRow>, sqlx::Error> {
    let folder_prefix = folder.map(|folder| format!("{}/%", openmemory_ops::escape_like(folder)));
    sqlx::query_as!(
        FilePathRow,
        r#"SELECT
  f.id AS "id: FileId",
  f.path,
  u.username AS author,
  f.created_at
FROM files AS f
JOIN users AS u ON u.id = f.user_id
WHERE f.deleted_at IS NULL
  AND f.path ILIKE $1
  AND ($2 OR f.path = ANY($3) OR f.path LIKE ANY($4))
  AND ($5::text IS NULL OR f.path = $5 OR f.path LIKE $6)
ORDER BY f.created_at DESC, f.id DESC
LIMIT $7"#,
        like_pattern,
        scope.unrestricted,
        &scope.exact,
        &scope.patterns,
        folder,
        folder_prefix,
        limit
    )
    .fetch_all(conn)
    .await
}
