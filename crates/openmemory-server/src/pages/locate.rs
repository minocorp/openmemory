use openmemory_ops::{PagePath, escape_like};
use sqlx::{PgConnection, PgPool};

use crate::access::Actor;
use crate::pages::error::PagesError;
use crate::pages::guard;
use crate::pages::ids::PageId;
use crate::pages::queries::pages::{self, PageRow};
use crate::purge::{self, Subject};

pub(super) async fn lock_live(
    conn: &mut PgConnection,
    path: &PagePath,
) -> Result<PageRow, PagesError> {
    pages::lock_live_by_path(conn, path.as_str())
        .await?
        .ok_or(PagesError::NotFound)
}

pub(super) async fn lock_live_by_id(
    conn: &mut PgConnection,
    id: PageId,
) -> Result<PageRow, PagesError> {
    pages::lock_by_id(conn, id)
        .await?
        .filter(|row| row.deleted_at.is_none())
        .ok_or(PagesError::NotFound)
}

pub(crate) async fn locate(
    pool: &PgPool,
    actor: &Actor,
    id: PageId,
) -> Result<PageRow, PagesError> {
    let mut conn = pool.acquire().await?;
    let Some(row) = pages::find_by_id(&mut conn, id).await? else {
        return if purge::is_purged(&mut conn, Subject::Page, id.0).await? {
            Err(PagesError::Purged)
        } else {
            Err(PagesError::NotFound)
        };
    };
    let path = PagePath::parse(&row.path).map_err(PagesError::StoredPath)?;
    if !guard::can_read(actor, &path) {
        return Err(PagesError::NotFound);
    }
    Ok(row)
}

pub(crate) async fn locate_live(
    pool: &PgPool,
    actor: &Actor,
    id: PageId,
) -> Result<PageRow, PagesError> {
    let row = locate(pool, actor, id).await?;
    if row.deleted_at.is_some() {
        return Err(PagesError::NotFound);
    }
    Ok(row)
}

pub(super) struct Locked {
    pub(super) page: PageRow,
    pub(super) inbound: Vec<PageRow>,
}

// Locks the page and every page that links to it in id order so concurrent moves cannot deadlock.
pub(super) async fn lock_with_inbound(
    conn: &mut PgConnection,
    id: PageId,
    path: &str,
) -> Result<Locked, PagesError> {
    let pattern = format!("%[[{}]]%", escape_like(path));
    let mut ids = pages::inbound_candidates(conn, id, &pattern).await?;
    ids.push(id.0);
    let mut rows = pages::lock_many(conn, &ids).await?;
    let position = rows
        .iter()
        .position(|row| row.id == id)
        .ok_or(PagesError::NotFound)?;
    let page = rows.remove(position);
    Ok(Locked {
        page,
        inbound: rows,
    })
}
