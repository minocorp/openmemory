use openmemory_ops::{BlockReason, BlockedPage, PagePath, RevertOutput, VersionOp};
use sqlx::{PgConnection, PgPool};

use crate::access::Actor;
use crate::pages::commit::{self, NextState};
use crate::pages::error::PagesError;
use crate::pages::group::{self, GroupJoin};
use crate::pages::guard;
use crate::pages::ids::{GroupId, PageId, VersionNumber};
use crate::pages::locate::{self, Locked, lock_with_inbound};
use crate::pages::queries::groups::{self, GroupPage};
use crate::pages::queries::pages::{self, PageRow};
use crate::pages::queries::versions::{self, VersionRow};
use crate::pages::relocate::rewrite_inbound;

#[derive(Debug)]
struct TargetState {
    path: String,
    content: String,
    is_deleted: bool,
    restored: VersionNumber,
}

impl TargetState {
    fn from_version(version: &VersionRow) -> Self {
        Self {
            path: version.path.clone(),
            content: version.content.clone(),
            is_deleted: version.op == VersionOp::Delete.as_str(),
            restored: version.n,
        }
    }

    fn matches(&self, page: &PageRow) -> bool {
        self.path == page.path
            && self.content == page.content
            && self.is_deleted == page.deleted_at.is_some()
    }

    fn needs_free_path(&self, page: &PageRow) -> bool {
        !self.is_deleted && (page.deleted_at.is_some() || page.path != self.path)
    }

    fn next(&self) -> NextState<'_> {
        NextState {
            path: &self.path,
            content: &self.content,
            is_deleted: self.is_deleted,
            op: VersionOp::Revert,
            reverts_version: Some(self.restored),
        }
    }
}

pub(crate) async fn revert_version(
    pool: &PgPool,
    actor: &Actor,
    path: &PagePath,
    version: VersionNumber,
    task: Option<&str>,
) -> Result<RevertOutput, PagesError> {
    guard::require_write(actor, path)?;
    revert_subject(pool, actor, Subject::Path(path), None, version, task).await
}

pub(crate) async fn revert_page_version(
    pool: &PgPool,
    actor: &Actor,
    id: PageId,
    expected: VersionNumber,
    version: VersionNumber,
    task: Option<&str>,
) -> Result<RevertOutput, PagesError> {
    let row = locate::locate(pool, actor, id).await?;
    guard::require_write_visible(actor, &row.path)?;
    revert_subject(pool, actor, Subject::Id(id), Some(expected), version, task).await
}

#[derive(Debug, Clone, Copy)]
enum Subject<'a> {
    Path(&'a PagePath),
    Id(PageId),
}

async fn revert_subject(
    pool: &PgPool,
    actor: &Actor,
    subject: Subject<'_>,
    expected: Option<VersionNumber>,
    version: VersionNumber,
    task: Option<&str>,
) -> Result<RevertOutput, PagesError> {
    let task = guard::task(task)?;
    let mut tx = pool.begin().await?;
    let group = group::assign(&mut tx, actor, task, GroupJoin::New).await?;
    let found = match subject {
        Subject::Path(path) => pages::find_latest_by_path(&mut tx, path.as_str()).await?,
        Subject::Id(id) => pages::find_by_id(&mut tx, id).await?,
    }
    .ok_or(PagesError::NotFound)?;
    if let Some(expected) = expected {
        guard::require_version(&found, expected)?;
    }
    let stored = versions::find(&mut tx, found.id, version)
        .await?
        .ok_or(PagesError::VersionNotFound)?;
    let target = TargetState::from_version(&stored);
    guard::require_write(actor, &parse_stored(&target.path)?)?;
    let rewrites_links =
        found.deleted_at.is_none() && !target.is_deleted && found.path != target.path;
    let locked = if rewrites_links {
        lock_with_inbound(&mut tx, found.id, &found.path).await?
    } else {
        lock_single(&mut tx, &found).await?
    };
    if locked.page.current_version != found.current_version {
        return Err(PagesError::Conflict {
            current_version: locked.page.current_version,
        });
    }
    if target.matches(&locked.page) {
        return Err(PagesError::AlreadyCurrent);
    }
    if target.needs_free_path(&locked.page)
        && pages::path_in_use(&mut tx, &target.path, Some(locked.page.id)).await?
    {
        return Err(PagesError::PathTaken);
    }
    let mut reverted =
        vec![commit::commit(&mut tx, actor, group, &locked.page, &target.next()).await?];
    if rewrites_links {
        let from = parse_stored(&locked.page.path)?;
        let to = parse_stored(&target.path)?;
        let rewritten = rewrite_inbound(&mut tx, actor, group, &locked.inbound, &from, &to).await?;
        reverted.extend(rewritten.visible);
    }
    tx.commit().await?;
    Ok(RevertOutput { reverted })
}

pub(crate) async fn revert_group(
    pool: &PgPool,
    actor: &Actor,
    group_id: GroupId,
    task: Option<&str>,
) -> Result<RevertOutput, PagesError> {
    let task = guard::task(task)?;
    let mut tx = pool.begin().await?;
    let group = group::assign(&mut tx, actor, task, GroupJoin::New).await?;
    let members = groups::pages_in_group(&mut tx, group_id).await?;
    if members.is_empty() {
        return Err(PagesError::GroupNotFound);
    }
    let ids: Vec<i64> = members.iter().map(|member| member.page_id.0).collect();
    let rows = pages::lock_many(&mut tx, &ids).await?;
    let mut plans = Vec::new();
    for (member, row) in members.iter().zip(&rows) {
        plans.push(plan(&mut tx, member, row).await?);
    }
    require_group_scope(actor, &plans)?;
    let blocked = blockers(&mut tx, &plans).await?;
    if !blocked.is_empty() {
        return Err(PagesError::RevertBlocked(blocked));
    }
    let mut reverted = Vec::new();
    for plan in plans.iter().filter(|plan| !plan.target.matches(plan.row)) {
        reverted.push(commit::commit(&mut tx, actor, group, plan.row, &plan.target.next()).await?);
    }
    if reverted.is_empty() {
        return Err(PagesError::AlreadyCurrent);
    }
    tx.commit().await?;
    Ok(RevertOutput { reverted })
}

struct Plan<'a> {
    row: &'a PageRow,
    last: VersionNumber,
    target: TargetState,
}

async fn plan<'a>(
    conn: &mut PgConnection,
    member: &GroupPage,
    row: &'a PageRow,
) -> Result<Plan<'a>, PagesError> {
    let before = if member.first.0 > 1 {
        versions::find(conn, member.page_id, VersionNumber(member.first.0 - 1)).await?
    } else {
        None
    };
    let target = match before {
        Some(version) => TargetState::from_version(&version),
        None => TargetState {
            path: row.path.clone(),
            content: row.content.clone(),
            is_deleted: true,
            restored: member.first,
        },
    };
    Ok(Plan {
        row,
        last: member.last,
        target,
    })
}

fn require_group_scope(actor: &Actor, plans: &[Plan<'_>]) -> Result<(), PagesError> {
    let mut paths = Vec::new();
    for plan in plans {
        paths.push(parse_stored(&plan.row.path)?);
        paths.push(parse_stored(&plan.target.path)?);
    }
    if !paths.iter().any(|path| guard::can_read(actor, path)) {
        return Err(PagesError::GroupNotFound);
    }
    paths
        .iter()
        .try_for_each(|path| guard::require_write(actor, path))
}

async fn blockers(
    conn: &mut PgConnection,
    plans: &[Plan<'_>],
) -> Result<Vec<BlockedPage>, PagesError> {
    let mut blocked = Vec::new();
    for plan in plans {
        if plan.target.matches(plan.row) {
            continue;
        }
        if plan.row.current_version > plan.last {
            blocked.push(BlockedPage {
                path: plan.row.path.clone(),
                reason: BlockReason::ChangedSince,
            });
        } else if plan.target.needs_free_path(plan.row)
            && pages::path_in_use(conn, &plan.target.path, Some(plan.row.id)).await?
        {
            blocked.push(BlockedPage {
                path: plan.target.path.clone(),
                reason: BlockReason::PathTaken,
            });
        }
    }
    Ok(blocked)
}

async fn lock_single(conn: &mut PgConnection, found: &PageRow) -> Result<Locked, PagesError> {
    let page = pages::lock_many(conn, &[found.id.0])
        .await?
        .pop()
        .ok_or(PagesError::NotFound)?;
    Ok(Locked {
        page,
        inbound: Vec::new(),
    })
}

fn parse_stored(path: &str) -> Result<PagePath, PagesError> {
    PagePath::parse(path).map_err(PagesError::StoredPath)
}
