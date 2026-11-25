use openmemory_ops::{PagePath, quote_occurs};
use sqlx::PgConnection;

use crate::access::Actor;
use crate::pages::error::{PagesError, QuoteProblem};

pub(super) async fn verify(
    conn: &mut PgConnection,
    actor: &Actor,
    path: &PagePath,
    page: Option<u32>,
    quote: &str,
) -> Result<(), PagesError> {
    match check(conn, actor, path, page, quote).await? {
        None => Ok(()),
        Some(problem) => Err(PagesError::QuoteUnverified {
            problem,
            path: path.to_string(),
            page,
        }),
    }
}

async fn check(
    conn: &mut PgConnection,
    actor: &Actor,
    path: &PagePath,
    page: Option<u32>,
    quote: &str,
) -> Result<Option<QuoteProblem>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"SELECT
  p.page,
  p.text
FROM files AS f
JOIN file_pages AS p ON p.file_id = f.id
WHERE f.path = $1
  AND f.deleted_at IS NULL
ORDER BY p.page"#,
        path.as_str()
    )
    .fetch_all(&mut *conn)
    .await?;
    let exists = !rows.is_empty() || file_exists(conn, path).await?;
    if !exists || !actor.scope.can_read(path) {
        return Ok(Some(QuoteProblem::NoFile));
    }
    let wanted = page.and_then(|page| i32::try_from(page).ok());
    let candidates: Vec<&str> = rows
        .iter()
        .filter(|row| page.is_none() || Some(row.page) == wanted)
        .map(|row| row.text.as_str())
        .collect();
    if page.is_some() && candidates.is_empty() && !rows.is_empty() {
        return Ok(Some(QuoteProblem::NoPage));
    }
    if candidates.iter().all(|text| text.trim().is_empty()) {
        return Ok(Some(QuoteProblem::NoText));
    }
    if candidates.iter().any(|text| quote_occurs(text, quote)) {
        Ok(None)
    } else {
        Ok(Some(QuoteProblem::NotInText))
    }
}

async fn file_exists(conn: &mut PgConnection, path: &PagePath) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT EXISTS (SELECT 1 FROM files WHERE path = $1 AND deleted_at IS NULL) AS "exists!""#,
        path.as_str()
    )
    .fetch_one(conn)
    .await
}
