use memory_ops::{PagePath, VersionOp, WriteOutput};
use sqlx::{PgConnection, PgPool};

use crate::access::Actor;
use crate::pages::commit::{self, NextState};
use crate::pages::error::PagesError;
use crate::pages::group::{self, GroupJoin};
use crate::pages::guard;
use crate::pages::ids::{GroupId, PageId, VersionNumber};
use crate::pages::locate::{lock_live, lock_live_by_id};
use crate::pages::queries::pages::PageRow;

pub(crate) async fn delete(
    pool: &PgPool,
    actor: &Actor,
    path: &PagePath,
    task: Option<&str>,
) -> Result<WriteOutput, PagesError> {
    guard::require_write(actor, path)?;
    guard::require_not_start(path)?;
    let task = guard::task(task)?;
    let mut tx = pool.begin().await?;
    let group = group::assign(&mut tx, actor, task, GroupJoin::Continue).await?;
    let row = lock_live(&mut tx, path).await?;
    let written = tombstone(&mut tx, actor, group, &row).await?;
    tx.commit().await?;
    Ok(written)
}

pub(crate) async fn delete_page(
    pool: &PgPool,
    actor: &Actor,
    id: PageId,
    expected: VersionNumber,
    task: Option<&str>,
) -> Result<WriteOutput, PagesError> {
    let task = guard::task(task)?;
    let mut tx = pool.begin().await?;
    let group = group::assign(&mut tx, actor, task, GroupJoin::Continue).await?;
    let row = lock_live_by_id(&mut tx, id).await?;
    guard::require_write_visible(actor, &row.path)?;
    guard::require_not_start(&PagePath::parse(&row.path).map_err(PagesError::StoredPath)?)?;
    guard::require_version(&row, expected)?;
    let written = tombstone(&mut tx, actor, group, &row).await?;
    tx.commit().await?;
    Ok(written)
}

async fn tombstone(
    conn: &mut PgConnection,
    actor: &Actor,
    group: GroupId,
    row: &PageRow,
) -> Result<WriteOutput, PagesError> {
    let next = NextState {
        path: &row.path,
        content: &row.content,
        is_deleted: true,
        op: VersionOp::Delete,
        reverts_version: None,
    };
    commit::commit(conn, actor, group, row, &next).await
}
