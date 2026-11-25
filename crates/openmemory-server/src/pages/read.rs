use openmemory_ops::{Page, PagePath, PageRead};
use sqlx::{PgConnection, PgPool};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::access::Actor;
use crate::pages::error::PagesError;
use crate::pages::guard;
use crate::pages::ids::{PageId, VersionNumber};
use crate::pages::locate;
use crate::pages::queries::pages::{self, PageRow};
use crate::pages::queries::versions;

pub(crate) async fn read(
    pool: &PgPool,
    actor: &Actor,
    path: &PagePath,
    version: Option<VersionNumber>,
) -> Result<PageRead, PagesError> {
    read_with_id(pool, actor, path, version)
        .await
        .map(|(_, page)| page)
}

pub(crate) async fn read_with_id(
    pool: &PgPool,
    actor: &Actor,
    path: &PagePath,
    version: Option<VersionNumber>,
) -> Result<(PageId, PageRead), PagesError> {
    if !guard::can_read(actor, path) {
        return Err(PagesError::NotFound);
    }
    let mut conn = pool.acquire().await?;
    let row = pages::find_live_by_path(&mut conn, path.as_str())
        .await?
        .ok_or(PagesError::NotFound)?;
    let page = render(&mut conn, actor, &row, version).await?;
    Ok((row.id, page))
}

pub(crate) async fn read_by_id(
    pool: &PgPool,
    actor: &Actor,
    id: PageId,
    version: Option<VersionNumber>,
) -> Result<PageRead, PagesError> {
    let row = locate::locate_live(pool, actor, id).await?;
    let mut conn = pool.acquire().await?;
    render(&mut conn, actor, &row, version).await
}

async fn render(
    conn: &mut PgConnection,
    actor: &Actor,
    row: &PageRow,
    version: Option<VersionNumber>,
) -> Result<PageRead, PagesError> {
    let n = version.unwrap_or(row.current_version);
    let stored = versions::find(conn, row.id, n)
        .await?
        .ok_or(PagesError::VersionNotFound)?;
    let stored_path = PagePath::parse(&stored.path).map_err(PagesError::StoredPath)?;
    if !guard::can_read(actor, &stored_path) {
        return Err(PagesError::VersionNotFound);
    }
    let page = Page::parse(&stored.content)?;
    Ok(PageRead::new(
        &row.path,
        n.0,
        &stored.author_name,
        &format_time(stored.created_at),
        &page,
    ))
}

pub(crate) fn format_time(at: OffsetDateTime) -> String {
    at.format(&Rfc3339).unwrap_or_default()
}
