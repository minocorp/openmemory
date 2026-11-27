use sqlx::PgConnection;

use crate::files::FileId;
use crate::pages::ids::PageId;

#[derive(Debug)]
pub(crate) struct PageText {
    pub(crate) id: PageId,
    pub(crate) content: String,
}

#[derive(Debug)]
pub(crate) struct FilePageText {
    pub(crate) file_id: FileId,
    pub(crate) page: i32,
    pub(crate) text: String,
}

pub(crate) async fn pages(
    conn: &mut PgConnection,
    ids: &[i64],
) -> Result<Vec<PageText>, sqlx::Error> {
    sqlx::query_as!(
        PageText,
        r#"SELECT
  p.id AS "id: PageId",
  p.content
FROM pages AS p
WHERE p.id = ANY($1)"#,
        ids
    )
    .fetch_all(conn)
    .await
}

pub(crate) async fn first_matching_file_pages(
    conn: &mut PgConnection,
    file_ids: &[i64],
    like_patterns: &[String],
) -> Result<Vec<FilePageText>, sqlx::Error> {
    sqlx::query_as!(
        FilePageText,
        r#"SELECT DISTINCT ON (fp.file_id)
  fp.file_id AS "file_id: FileId",
  fp.page,
  fp.text
FROM file_pages AS fp
WHERE fp.file_id = ANY($1)
  AND search_text(fp.text) LIKE ANY (SELECT search_text(pattern) FROM unnest($2::text[]) AS pattern)
ORDER BY fp.file_id, fp.page"#,
        file_ids,
        like_patterns
    )
    .fetch_all(conn)
    .await
}
