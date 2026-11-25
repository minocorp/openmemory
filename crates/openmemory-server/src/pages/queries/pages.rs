use sqlx::PgConnection;
use time::OffsetDateTime;

use crate::pages::ids::{PageId, VersionNumber};

#[derive(Debug, Clone)]
pub(crate) struct PageRow {
    pub(crate) id: PageId,
    pub(crate) path: String,
    pub(crate) current_version: VersionNumber,
    pub(crate) content: String,
    pub(crate) deleted_at: Option<OffsetDateTime>,
}

pub(crate) async fn lock_live_by_path(
    conn: &mut PgConnection,
    path: &str,
) -> Result<Option<PageRow>, sqlx::Error> {
    sqlx::query_as!(
        PageRow,
        r#"SELECT
  id AS "id: PageId",
  path,
  current_version AS "current_version: VersionNumber",
  content,
  deleted_at
FROM pages
WHERE path = $1
  AND deleted_at IS NULL
FOR UPDATE"#,
        path
    )
    .fetch_optional(conn)
    .await
}

pub(crate) async fn lock_many(
    conn: &mut PgConnection,
    ids: &[i64],
) -> Result<Vec<PageRow>, sqlx::Error> {
    sqlx::query_as!(
        PageRow,
        r#"SELECT
  id AS "id: PageId",
  path,
  current_version AS "current_version: VersionNumber",
  content,
  deleted_at
FROM pages
WHERE id = ANY($1)
ORDER BY id
FOR UPDATE"#,
        ids
    )
    .fetch_all(conn)
    .await
}

pub(crate) async fn find_live_by_path(
    conn: &mut PgConnection,
    path: &str,
) -> Result<Option<PageRow>, sqlx::Error> {
    sqlx::query_as!(
        PageRow,
        r#"SELECT
  id AS "id: PageId",
  path,
  current_version AS "current_version: VersionNumber",
  content,
  deleted_at
FROM pages
WHERE path = $1
  AND deleted_at IS NULL"#,
        path
    )
    .fetch_optional(conn)
    .await
}

pub(crate) async fn find_latest_by_path(
    conn: &mut PgConnection,
    path: &str,
) -> Result<Option<PageRow>, sqlx::Error> {
    sqlx::query_as!(
        PageRow,
        r#"SELECT
  id AS "id: PageId",
  path,
  current_version AS "current_version: VersionNumber",
  content,
  deleted_at
FROM pages
WHERE path = $1
ORDER BY (deleted_at IS NULL) DESC, id DESC
LIMIT 1"#,
        path
    )
    .fetch_optional(conn)
    .await
}

pub(crate) async fn find_by_id(
    conn: &mut PgConnection,
    id: PageId,
) -> Result<Option<PageRow>, sqlx::Error> {
    sqlx::query_as!(
        PageRow,
        r#"SELECT
  id AS "id: PageId",
  path,
  current_version AS "current_version: VersionNumber",
  content,
  deleted_at
FROM pages
WHERE id = $1"#,
        id as PageId
    )
    .fetch_optional(conn)
    .await
}

pub(crate) async fn lock_by_id(
    conn: &mut PgConnection,
    id: PageId,
) -> Result<Option<PageRow>, sqlx::Error> {
    sqlx::query_as!(
        PageRow,
        r#"SELECT
  id AS "id: PageId",
  path,
  current_version AS "current_version: VersionNumber",
  content,
  deleted_at
FROM pages
WHERE id = $1
FOR UPDATE"#,
        id as PageId
    )
    .fetch_optional(conn)
    .await
}

pub(crate) async fn delete_by_id(conn: &mut PgConnection, id: PageId) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM pages WHERE id = $1", id as PageId)
        .execute(conn)
        .await?;
    Ok(())
}

pub(crate) async fn insert(
    conn: &mut PgConnection,
    path: &str,
    content: &str,
) -> Result<PageId, sqlx::Error> {
    sqlx::query_scalar!(
        r#"INSERT INTO pages (path, current_version, content)
VALUES ($1, 1, $2)
RETURNING id AS "id: PageId""#,
        path,
        content
    )
    .fetch_one(conn)
    .await
}

pub(crate) async fn update_if_current(
    conn: &mut PgConnection,
    id: PageId,
    expected: VersionNumber,
    next: VersionNumber,
    path: &str,
    content: &str,
    is_deleted: bool,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        r#"UPDATE pages
SET current_version = $3,
  path = $4,
  content = $5,
  deleted_at = CASE WHEN $6 THEN now() ELSE NULL END
WHERE id = $1
  AND current_version = $2"#,
        id as PageId,
        expected as VersionNumber,
        next as VersionNumber,
        path,
        content,
        is_deleted
    )
    .execute(conn)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub(crate) async fn path_in_use(
    conn: &mut PgConnection,
    path: &str,
    except: Option<PageId>,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT (
  EXISTS (
    SELECT 1
    FROM pages
    WHERE path = $1
      AND deleted_at IS NULL
      AND id IS DISTINCT FROM $2
  )
  OR EXISTS (
    SELECT 1
    FROM files
    WHERE path = $1
      AND deleted_at IS NULL
  )
) AS "in_use!""#,
        path,
        except as Option<PageId>
    )
    .fetch_one(conn)
    .await
}

pub(crate) async fn inbound_candidates(
    conn: &mut PgConnection,
    target: PageId,
    link_pattern: &str,
) -> Result<Vec<i64>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT p.id AS "id!"
FROM pages AS p
WHERE p.deleted_at IS NULL
  AND p.id <> $1
  AND (
    p.id IN (SELECT l.from_page_id FROM page_links AS l WHERE l.to_page_id = $1)
    OR p.content LIKE $2
  )
ORDER BY p.id"#,
        target as PageId,
        link_pattern
    )
    .fetch_all(conn)
    .await
}
