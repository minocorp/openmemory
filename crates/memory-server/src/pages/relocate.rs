use memory_ops::{MoveOutput, PagePath, VersionOp, WriteOutput, rewrite_links};
use sqlx::{PgConnection, PgPool};

use crate::access::Actor;
use crate::pages::commit::{self, NextState};
use crate::pages::error::PagesError;
use crate::pages::group::{self, GroupJoin};
use crate::pages::guard;
use crate::pages::ids::{GroupId, PageId, VersionNumber};
use crate::pages::locate::{self, lock_with_inbound};
use crate::pages::queries::pages::{self, PageRow};

pub(crate) async fn move_page(
    pool: &PgPool,
    actor: &Actor,
    from: &PagePath,
    to: &PagePath,
    task: Option<&str>,
) -> Result<MoveOutput, PagesError> {
    guard::require_write(actor, from)?;
    guard::require_write(actor, to)?;
    guard::require_not_start(from)?;
    guard::require_not_start(to)?;
    if from == to {
        return Err(PagesError::SamePath);
    }
    let task = guard::task(task)?;
    let mut tx = pool.begin().await?;
    let group = group::assign(&mut tx, actor, task, GroupJoin::Continue).await?;
    let found = pages::find_live_by_path(&mut tx, from.as_str())
        .await?
        .ok_or(PagesError::NotFound)?;
    let moved = relocate(&mut tx, actor, group, found.id, (from, to), None).await?;
    tx.commit().await?;
    Ok(moved)
}

pub(crate) async fn move_page_by_id(
    pool: &PgPool,
    actor: &Actor,
    id: PageId,
    expected: VersionNumber,
    to: &PagePath,
    task: Option<&str>,
) -> Result<MoveOutput, PagesError> {
    let found = locate::locate_live(pool, actor, id).await?;
    let from = PagePath::parse(&found.path).map_err(PagesError::StoredPath)?;
    guard::require_write(actor, &from)?;
    guard::require_write(actor, to)?;
    guard::require_not_start(&from)?;
    guard::require_not_start(to)?;
    if &from == to {
        return Err(PagesError::SamePath);
    }
    let task = guard::task(task)?;
    let mut tx = pool.begin().await?;
    let group = group::assign(&mut tx, actor, task, GroupJoin::Continue).await?;
    let moved = relocate(&mut tx, actor, group, id, (&from, to), Some(expected)).await?;
    tx.commit().await?;
    Ok(moved)
}

async fn relocate(
    conn: &mut PgConnection,
    actor: &Actor,
    group: GroupId,
    id: PageId,
    (from, to): (&PagePath, &PagePath),
    expected: Option<VersionNumber>,
) -> Result<MoveOutput, PagesError> {
    let locked = lock_with_inbound(conn, id, from.as_str()).await?;
    if locked.page.deleted_at.is_some() {
        return Err(PagesError::NotFound);
    }
    if let Some(expected) = expected {
        guard::require_version(&locked.page, expected)?;
    }
    if locked.page.path != from.as_str() {
        return Err(PagesError::NotFound);
    }
    if pages::path_in_use(conn, to.as_str(), None).await? {
        return Err(PagesError::PathTaken);
    }
    let moved = commit_move(conn, actor, group, &locked.page, from, to).await?;
    let rewritten = rewrite_inbound(conn, actor, group, &locked.inbound, from, to).await?;
    Ok(MoveOutput {
        path: moved.path,
        version: moved.version,
        rewritten: rewritten.visible,
        rewritten_out_of_scope: rewritten.out_of_scope,
    })
}

pub(super) async fn commit_move(
    conn: &mut PgConnection,
    actor: &Actor,
    group: GroupId,
    page: &PageRow,
    from: &PagePath,
    to: &PagePath,
) -> Result<WriteOutput, PagesError> {
    let content = rewrite_links(&page.content, from, to);
    let next = NextState {
        path: to.as_str(),
        content: content.as_deref().unwrap_or(&page.content),
        is_deleted: false,
        op: VersionOp::Move,
        reverts_version: None,
    };
    commit::commit(conn, actor, group, page, &next).await
}

pub(super) struct Rewritten {
    pub(super) visible: Vec<WriteOutput>,
    pub(super) out_of_scope: u32,
}

pub(super) async fn rewrite_inbound(
    conn: &mut PgConnection,
    actor: &Actor,
    group: GroupId,
    inbound: &[PageRow],
    from: &PagePath,
    to: &PagePath,
) -> Result<Rewritten, PagesError> {
    let linker = Actor {
        name: format!("{} (link update)", actor.name),
        ..actor.clone()
    };
    let mut rewritten = Rewritten {
        visible: Vec::new(),
        out_of_scope: 0,
    };
    for page in inbound {
        let Some(content) = rewrite_links(&page.content, from, to) else {
            continue;
        };
        let next = NextState {
            path: &page.path,
            content: &content,
            is_deleted: false,
            op: VersionOp::Relink,
            reverts_version: None,
        };
        let written = commit::commit(conn, &linker, group, page, &next).await?;
        let readable = PagePath::parse(&page.path)
            .map_err(PagesError::StoredPath)
            .map(|path| guard::can_read(actor, &path))?;
        if readable {
            rewritten.visible.push(written);
        } else {
            rewritten.out_of_scope += 1;
        }
    }
    Ok(rewritten)
}
