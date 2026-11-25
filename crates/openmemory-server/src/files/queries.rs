use openmemory_ops::PagePath;
use sqlx::PgConnection;
use time::OffsetDateTime;

use crate::access::UserId;
use crate::files::{FileId, FileRecord, FilesError};
use crate::pages::scope_filter::ScopeFilter;
use crate::storage::Sha256Hash;

pub(super) struct FileRow {
    id: FileId,
    path: String,
    sha256: Vec<u8>,
    mime: String,
    size: i64,
    user_id: UserId,
    created_at: OffsetDateTime,
    page_count: i64,
}

pub(super) struct NewFile<'a> {
    pub(super) path: &'a PagePath,
    pub(super) sha256: &'a Sha256Hash,
    pub(super) blob_key: &'a str,
    pub(super) mime: &'a str,
    pub(super) size: i64,
    pub(super) user_id: UserId,
}

impl TryFrom<FileRow> for FileRecord {
    type Error = FilesError;

    fn try_from(row: FileRow) -> Result<Self, Self::Error> {
        Ok(Self {
            id: row.id,
            path: PagePath::parse(&row.path).map_err(|_| FilesError::StoredRowInvalid)?,
            sha256: Sha256Hash::try_from(row.sha256).map_err(|_| FilesError::StoredRowInvalid)?,
            mime: row.mime,
            size: row.size,
            uploader: row.user_id,
            created_at: row.created_at,
            page_count: row.page_count,
        })
    }
}

pub(super) async fn find_live_by_path(
    conn: &mut PgConnection,
    path: &PagePath,
) -> Result<Option<FileRow>, sqlx::Error> {
    sqlx::query_as!(
        FileRow,
        r#"SELECT
  f.id AS "id: FileId",
  f.path,
  f.sha256,
  f.mime,
  f.size,
  f.user_id AS "user_id: UserId",
  f.created_at,
  (SELECT count(*) FROM file_pages AS p WHERE p.file_id = f.id) AS "page_count!"
FROM files AS f
WHERE f.path = $1
  AND f.deleted_at IS NULL"#,
        path.as_str(),
    )
    .fetch_optional(conn)
    .await
}

pub(super) async fn find_live_by_id(
    conn: &mut PgConnection,
    id: FileId,
) -> Result<Option<FileRow>, sqlx::Error> {
    sqlx::query_as!(
        FileRow,
        r#"SELECT
  f.id AS "id: FileId",
  f.path,
  f.sha256,
  f.mime,
  f.size,
  f.user_id AS "user_id: UserId",
  f.created_at,
  (SELECT count(*) FROM file_pages AS p WHERE p.file_id = f.id) AS "page_count!"
FROM files AS f
WHERE f.id = $1
  AND f.deleted_at IS NULL"#,
        id.0,
    )
    .fetch_optional(conn)
    .await
}

pub(super) async fn ids_for_paths(
    conn: &mut PgConnection,
    paths: &[String],
) -> Result<Vec<(String, FileId)>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"SELECT
  path,
  id AS "id: FileId"
FROM files
WHERE path = ANY($1)
  AND deleted_at IS NULL"#,
        paths
    )
    .fetch_all(conn)
    .await?;
    Ok(rows.into_iter().map(|row| (row.path, row.id)).collect())
}

pub(super) async fn path_is_taken(
    conn: &mut PgConnection,
    path: &PagePath,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT
  EXISTS (SELECT 1 FROM pages WHERE path = $1 AND deleted_at IS NULL)
  OR EXISTS (SELECT 1 FROM files WHERE path = $1 AND deleted_at IS NULL) AS "taken!""#,
        path.as_str(),
    )
    .fetch_one(conn)
    .await
}

pub(super) async fn insert_file(
    conn: &mut PgConnection,
    file: &NewFile<'_>,
) -> Result<(FileId, OffsetDateTime), sqlx::Error> {
    let row = sqlx::query!(
        r#"INSERT INTO files (path, sha256, blob_key, mime, size, user_id)
VALUES ($1, $2, $3, $4, $5, $6)
RETURNING id AS "id: FileId", created_at"#,
        file.path.as_str(),
        file.sha256.as_bytes(),
        file.blob_key,
        file.mime,
        file.size,
        file.user_id.0,
    )
    .fetch_one(conn)
    .await?;
    Ok((row.id, row.created_at))
}

pub(super) async fn insert_pages(
    conn: &mut PgConnection,
    file_id: FileId,
    pages: &[String],
) -> Result<(), sqlx::Error> {
    let numbers: Vec<i32> = (1..).take(pages.len()).collect();
    sqlx::query!(
        "INSERT INTO file_pages (file_id, page, text)
SELECT $1, page, text
FROM UNNEST($2::integer[], $3::text[]) AS t (page, text)",
        file_id.0,
        &numbers,
        pages,
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub(super) async fn page_text(
    conn: &mut PgConnection,
    file_id: FileId,
    page: i32,
) -> Result<Option<String>, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT text FROM file_pages WHERE file_id = $1 AND page = $2",
        file_id.0,
        page,
    )
    .fetch_optional(conn)
    .await
}

pub(super) async fn soft_delete(conn: &mut PgConnection, id: FileId) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        "UPDATE files SET deleted_at = now() WHERE id = $1 AND deleted_at IS NULL",
        id.0,
    )
    .execute(conn)
    .await?;
    Ok(result.rows_affected())
}

pub(super) async fn list_live(
    conn: &mut PgConnection,
    scope: &ScopeFilter,
) -> Result<Vec<FileRow>, sqlx::Error> {
    sqlx::query_as!(
        FileRow,
        r#"SELECT
  f.id AS "id: FileId",
  f.path,
  f.sha256,
  f.mime,
  f.size,
  f.user_id AS "user_id: UserId",
  f.created_at,
  (SELECT count(*) FROM file_pages AS p WHERE p.file_id = f.id) AS "page_count!"
FROM files AS f
WHERE f.deleted_at IS NULL
  AND ($1 OR f.path = ANY($2) OR f.path LIKE ANY($3))
ORDER BY f.path"#,
        scope.unrestricted,
        &scope.exact,
        &scope.patterns,
    )
    .fetch_all(conn)
    .await
}

pub(super) struct PurgedFile {
    pub(super) path: String,
    pub(super) blob_key: String,
}

pub(super) async fn lock_any_by_id(
    conn: &mut PgConnection,
    id: FileId,
) -> Result<Option<PurgedFile>, sqlx::Error> {
    sqlx::query_as!(
        PurgedFile,
        "SELECT path, blob_key FROM files WHERE id = $1 FOR UPDATE",
        id.0,
    )
    .fetch_optional(conn)
    .await
}

pub(super) async fn delete_by_id(conn: &mut PgConnection, id: FileId) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM files WHERE id = $1", id.0)
        .execute(conn)
        .await?;
    Ok(())
}

pub(super) async fn lock_blob(conn: &mut PgConnection, blob_key: &str) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "SELECT pg_advisory_xact_lock(hashtextextended($1, 0))",
        blob_key
    )
    .fetch_one(conn)
    .await?;
    Ok(())
}

pub(super) async fn blob_is_referenced(
    conn: &mut PgConnection,
    blob_key: &str,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM files WHERE blob_key = $1) AS "referenced!""#,
        blob_key,
    )
    .fetch_one(conn)
    .await
}

pub(crate) async fn referenced_blob_keys(
    conn: &mut PgConnection,
    keys: &[String],
) -> Result<Vec<String>, sqlx::Error> {
    sqlx::query_scalar!(
        "SELECT DISTINCT blob_key FROM files WHERE blob_key = ANY($1)",
        keys,
    )
    .fetch_all(conn)
    .await
}
