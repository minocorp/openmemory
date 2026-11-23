use std::collections::HashSet;

use memory_ops::{HitKind, SearchHit};
use sqlx::PgConnection;

use crate::files::FileId;
use crate::pages::read::format_time;
use crate::pages::scope_filter::ScopeFilter;
use crate::search::SearchError;
use crate::search::hits::matching_lines;
use crate::search::queries::files::{self, FileMatchRow, FilePathRow};

pub(super) struct FileQuery<'a> {
    pub(super) scope: &'a ScopeFilter,
    pub(super) folder: Option<&'a str>,
    pub(super) text: &'a str,
    pub(super) like_pattern: &'a str,
    pub(super) match_paths: bool,
    pub(super) limit: i64,
}

pub(super) async fn search_files(
    conn: &mut PgConnection,
    query: &FileQuery<'_>,
) -> Result<Vec<SearchHit>, SearchError> {
    let rows = files::find(
        conn,
        query.scope,
        query.folder,
        query.like_pattern,
        query.limit,
    )
    .await?;
    let mut seen: HashSet<FileId> = HashSet::new();
    let mut hits: Vec<SearchHit> = rows
        .into_iter()
        .filter_map(|row| content_hit(row, query.text))
        .inspect(|(id, _)| {
            seen.insert(*id);
        })
        .map(|(_, hit)| hit)
        .collect();
    if query.match_paths {
        let rows = files::find_by_path(
            conn,
            query.scope,
            query.folder,
            query.like_pattern,
            query.limit,
        )
        .await?;
        hits.extend(
            rows.into_iter()
                .filter(|row| !seen.contains(&row.id))
                .map(path_hit),
        );
    }
    Ok(hits)
}

fn content_hit(row: FileMatchRow, text: &str) -> Option<(FileId, SearchHit)> {
    let (line_number, line) = matching_lines(&row.text, text).into_iter().next()?;
    let hit = SearchHit {
        kind: HitKind::File,
        path: row.path,
        page: Some(u32::try_from(row.page).unwrap_or(1)),
        line_number: Some(line_number),
        line: Some(line),
        author: row.author,
        at: format_time(row.created_at),
    };
    Some((row.id, hit))
}

fn path_hit(row: FilePathRow) -> SearchHit {
    SearchHit {
        kind: HitKind::File,
        path: row.path,
        page: None,
        line_number: None,
        line: None,
        author: row.author,
        at: format_time(row.created_at),
    }
}
