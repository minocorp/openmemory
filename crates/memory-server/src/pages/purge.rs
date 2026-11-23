use memory_ops::PagePath;
use sqlx::PgPool;

use crate::access::UserId;
use crate::pages::error::PagesError;
use crate::pages::guard;
use crate::pages::ids::PageId;
use crate::pages::queries::pages;
use crate::purge::{self, Subject};

pub(crate) async fn purge(pool: &PgPool, user_id: UserId, id: PageId) -> Result<(), PagesError> {
    let mut tx = pool.begin().await?;
    let row = pages::lock_by_id(&mut tx, id)
        .await?
        .ok_or(PagesError::NotFound)?;
    guard::require_not_start(&PagePath::parse(&row.path).map_err(PagesError::StoredPath)?)?;
    purge::record(&mut tx, Subject::Page, id.0, &row.path, user_id).await?;
    pages::delete_by_id(&mut tx, id).await?;
    tx.commit().await?;
    Ok(())
}
