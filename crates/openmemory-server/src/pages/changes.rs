use openmemory_ops::VersionOp;
use serde::Serialize;
use sqlx::PgPool;

use crate::access::Actor;
use crate::pages::error::PagesError;
use crate::pages::history::unified_diff;
use crate::pages::ids::{GroupId, PageId, Seq};
use crate::pages::queries::changes::{self, ChangeRow};
use crate::pages::read::format_time;
use crate::pages::scope_filter::ScopeFilter;

const DEFAULT_LIMIT: u32 = 500;
const MAX_LIMIT: u32 = 2000;

#[derive(Debug, Serialize)]
pub(crate) struct Changes {
    pub(crate) groups: Vec<ChangeGroup>,
    pub(crate) last_seq: Seq,
    pub(crate) next: Option<Seq>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChangeGroup {
    pub(crate) id: GroupId,
    pub(crate) task: Option<String>,
    pub(crate) author: String,
    pub(crate) author_display_name: String,
    pub(crate) at: String,
    pub(crate) versions: Vec<ChangedVersion>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ChangedVersion {
    pub(crate) page_id: PageId,
    pub(crate) path: String,
    pub(crate) version: i32,
    pub(crate) op: VersionOp,
    pub(crate) seq: Seq,
    pub(crate) diff: String,
}

pub(crate) async fn list(
    pool: &PgPool,
    actor: &Actor,
    since: Seq,
    limit: Option<u32>,
) -> Result<Changes, PagesError> {
    let mut conn = pool.acquire().await?;
    let scope = ScopeFilter::of(&actor.scope);
    let limit = i64::from(limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT));
    let mut rows = changes::list_since(&mut conn, since, &scope, actor.user_id, limit + 1).await?;
    let has_more = rows.len() > usize::try_from(limit).unwrap_or(usize::MAX);
    rows.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
    let last_seq = rows.last().map_or(since, |row| row.seq);
    Ok(Changes {
        groups: group_rows(rows),
        last_seq,
        next: has_more.then_some(last_seq),
    })
}

pub(crate) async fn last_seen(pool: &PgPool, actor: &Actor) -> Result<Seq, PagesError> {
    let mut conn = pool.acquire().await?;
    Ok(changes::last_seen(&mut conn, actor.user_id)
        .await?
        .unwrap_or(Seq(0)))
}

pub(crate) async fn mark_seen(pool: &PgPool, actor: &Actor, seq: Seq) -> Result<Seq, PagesError> {
    let mut conn = pool.acquire().await?;
    Ok(changes::mark_seen(&mut conn, actor.user_id, seq)
        .await?
        .unwrap_or(Seq(0)))
}

fn group_rows(rows: Vec<ChangeRow>) -> Vec<ChangeGroup> {
    let mut groups: Vec<ChangeGroup> = Vec::new();
    for row in rows {
        let version = ChangedVersion {
            page_id: row.page_id,
            diff: unified_diff(
                row.previous_content.as_deref(),
                &row.content,
                &row.path,
                row.n.0,
            ),
            path: row.path,
            version: row.n.0,
            op: VersionOp::parse(&row.op).unwrap_or(VersionOp::Edit),
            seq: row.seq,
        };
        match groups.iter_mut().find(|group| group.id == row.group_id) {
            Some(group) => group.versions.push(version),
            None => groups.push(ChangeGroup {
                id: row.group_id,
                task: row.task,
                author: row.author_name,
                author_display_name: row.author_display_name,
                at: format_time(row.created_at),
                versions: vec![version],
            }),
        }
    }
    groups
}
