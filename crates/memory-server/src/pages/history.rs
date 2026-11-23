use memory_ops::{HistoryOutput, PagePath, VersionEntry, VersionOp};
use similar::TextDiff;
use sqlx::PgPool;
use time::Date;

use crate::access::{Actor, Scope};
use crate::pages::error::PagesError;
use crate::pages::guard;
use crate::pages::ids::PageId;
use crate::pages::locate;
use crate::pages::queries::history::{self, HistoryFilter, HistoryRow};
use crate::pages::queries::pages;
use crate::pages::read::format_time;
use crate::pages::scope_filter::ScopeFilter;

const DEFAULT_LIMIT: u32 = 20;
const MAX_LIMIT: u32 = 100;
const DIFF_CONTEXT_LINES: usize = 3;

#[derive(Debug, Default)]
pub(crate) struct HistoryQuery<'a> {
    pub(crate) path: Option<&'a PagePath>,
    pub(crate) by: Option<&'a str>,
    pub(crate) since: Option<Date>,
    pub(crate) limit: Option<u32>,
}

#[derive(Debug)]
pub(crate) struct VersionPage {
    pub(crate) versions: Vec<VersionEntry>,
    pub(crate) next_before: Option<i32>,
}

pub(crate) async fn history(
    pool: &PgPool,
    actor: &Actor,
    query: &HistoryQuery<'_>,
) -> Result<HistoryOutput, PagesError> {
    let mut conn = pool.acquire().await?;
    let page_id = match query.path {
        Some(path) => {
            if !guard::can_read(actor, path) {
                return Err(PagesError::NotFound);
            }
            let row = pages::find_latest_by_path(&mut conn, path.as_str())
                .await?
                .ok_or(PagesError::NotFound)?;
            Some(row.id.0)
        }
        None => None,
    };
    let scope = if query
        .path
        .is_some_and(|path| path.as_str() == guard::START_PATH)
    {
        ScopeFilter::of(&Scope::everything())
    } else {
        ScopeFilter::of(&actor.scope)
    };
    let filter = HistoryFilter {
        page_id,
        by: query.by,
        since: query.since.map(|date| date.midnight().assume_utc()),
        scope: &scope,
        limit: i64::from(query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT)),
        before_version: None,
    };
    let rows = history::list(&mut conn, &filter).await?;
    Ok(HistoryOutput {
        versions: rows.into_iter().map(entry).collect(),
    })
}

pub(crate) async fn versions_of_page(
    pool: &PgPool,
    actor: &Actor,
    id: PageId,
    limit: Option<u32>,
    before: Option<i32>,
) -> Result<VersionPage, PagesError> {
    let row = locate::locate(pool, actor, id).await?;
    let scope = if row.path == guard::START_PATH {
        ScopeFilter::of(&Scope::everything())
    } else {
        ScopeFilter::of(&actor.scope)
    };
    let limit = limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let filter = HistoryFilter {
        page_id: Some(id.0),
        by: None,
        since: None,
        scope: &scope,
        limit: i64::from(limit) + 1,
        before_version: before,
    };
    let mut conn = pool.acquire().await?;
    let mut rows = history::list(&mut conn, &filter).await?;
    let page_size = usize::try_from(limit).unwrap_or(usize::MAX);
    let has_more = rows.len() > page_size;
    rows.truncate(page_size);
    let next_before = if has_more {
        rows.last().map(|last| last.n.0)
    } else {
        None
    };
    Ok(VersionPage {
        versions: rows.into_iter().map(entry).collect(),
        next_before,
    })
}

pub(crate) fn unified_diff(previous: Option<&str>, content: &str, path: &str, n: i32) -> String {
    let old_label = if n > 1 {
        format!("{path}@v{}", n - 1)
    } else {
        "/dev/null".to_owned()
    };
    let new_label = format!("{path}@v{n}");
    TextDiff::from_lines(previous.unwrap_or_default(), content)
        .unified_diff()
        .context_radius(DIFF_CONTEXT_LINES)
        .header(&old_label, &new_label)
        .to_string()
}

fn entry(row: HistoryRow) -> VersionEntry {
    let diff = unified_diff(
        row.previous_content.as_deref(),
        &row.content,
        &row.path,
        row.n.0,
    );
    VersionEntry {
        path: row.path,
        version: row.n.0,
        op: VersionOp::parse(&row.op).unwrap_or(VersionOp::Edit),
        author: row.author_name,
        at: format_time(row.created_at),
        group: row.group_id.0,
        reverts_version: row.reverts_version.map(|version| version.0),
        diff,
    }
}
