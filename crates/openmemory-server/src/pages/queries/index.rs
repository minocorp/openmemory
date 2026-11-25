use sqlx::PgConnection;

use crate::pages::ids::PageId;

#[derive(Debug, Default)]
pub(crate) struct FieldRows {
    pub(crate) keys: Vec<String>,
    pub(crate) ords: Vec<i32>,
    pub(crate) texts: Vec<String>,
    pub(crate) numbers: Vec<String>,
    pub(crate) dates: Vec<String>,
}

pub(crate) async fn replace_fields(
    conn: &mut PgConnection,
    page_id: PageId,
    rows: &FieldRows,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM page_fields WHERE page_id = $1",
        page_id as PageId
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!(
        r#"INSERT INTO page_fields (page_id, key, ord, text, num, date)
SELECT
  $1,
  f.key,
  f.ord,
  f.text,
  NULLIF(f.num, '')::numeric,
  NULLIF(f.date, '')::date
FROM UNNEST($2::text[], $3::int[], $4::text[], $5::text[], $6::text[]) AS f (key, ord, text, num, date)"#,
        page_id as PageId,
        &rows.keys,
        &rows.ords,
        &rows.texts,
        &rows.numbers,
        &rows.dates
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub(crate) async fn resolve_page_targets(
    conn: &mut PgConnection,
    paths: &[String],
    except: PageId,
) -> Result<Vec<i64>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT id AS "id!"
FROM pages
WHERE path = ANY($1)
  AND deleted_at IS NULL
  AND id <> $2"#,
        paths,
        except as PageId
    )
    .fetch_all(conn)
    .await
}

pub(crate) async fn resolve_file_targets(
    conn: &mut PgConnection,
    paths: &[String],
) -> Result<Vec<i64>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT id AS "id!"
FROM files
WHERE path = ANY($1)
  AND deleted_at IS NULL"#,
        paths
    )
    .fetch_all(conn)
    .await
}

pub(crate) async fn replace_links(
    conn: &mut PgConnection,
    page_id: PageId,
    to_pages: &[i64],
    to_files: &[i64],
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "DELETE FROM page_links WHERE from_page_id = $1",
        page_id as PageId
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!(
        "INSERT INTO page_links (from_page_id, to_page_id) SELECT $1, UNNEST($2::bigint[])",
        page_id as PageId,
        to_pages
    )
    .execute(&mut *conn)
    .await?;
    sqlx::query!(
        "INSERT INTO page_links (from_page_id, to_file_id) SELECT $1, UNNEST($2::bigint[])",
        page_id as PageId,
        to_files
    )
    .execute(conn)
    .await?;
    Ok(())
}
