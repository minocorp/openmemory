mod error;
mod files;
mod hits;
mod queries;

#[cfg(test)]
mod tests;

use memory_ops::{
    NoHits, PagePath, ResolvedCondition, SearchInput, SearchOutput, WhereClause, escape_like,
};
use sqlx::PgPool;

use crate::access::Actor;
use crate::pages::scope_filter::ScopeFilter;
use crate::search::files::{FileQuery, search_files};
use crate::search::queries::matches::{self, MatchFilter};
use crate::search::queries::scope;
use crate::time_zone::TimeZone;

pub(crate) use error::SearchError;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Match {
    Text,
    TextAndPaths,
}

const DEFAULT_LIMIT: u32 = 20;
const MAX_LIMIT: u32 = 100;

pub(crate) async fn search(
    pool: &PgPool,
    time_zone: &TimeZone,
    actor: &Actor,
    input: &SearchInput,
    matching: Match,
) -> Result<SearchOutput, SearchError> {
    let mut conn = pool.acquire().await?;
    let today = time_zone.today(&mut conn).await?;
    let conditions = WhereClause::parse(&input.conditions)?.resolve(today)?;
    let folder = input
        .folder
        .as_deref()
        .map(PagePath::parse)
        .transpose()
        .map_err(SearchError::Folder)?;
    let text = input.text.as_deref().filter(|text| !text.is_empty());
    let like_pattern = text.map(|text| format!("%{}%", escape_like(text)));
    let scope_filter = ScopeFilter::of(&actor.scope);
    let limit = i64::from(input.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT));
    let filter = MatchFilter {
        scope: &scope_filter,
        folder: folder.as_ref().map(PagePath::as_str),
        like_pattern: like_pattern.as_deref(),
        match_paths: matching == Match::TextAndPaths,
        conditions: &conditions,
        before_seq: input.cursor,
        limit: limit + 1,
    };
    let mut rows = matches::find(&mut conn, &filter).await?;
    let has_more = rows.len() > usize::try_from(limit).unwrap_or(usize::MAX);
    rows.truncate(usize::try_from(limit).unwrap_or(usize::MAX));
    let next_cursor = if has_more {
        rows.last().map(|row| row.seq)
    } else {
        None
    };
    let mut found = rows
        .iter()
        .flat_map(|row| hits::hits_for(row, text))
        .collect::<Vec<_>>();
    if let Some(text) = text
        && let Some(like_pattern) = like_pattern.as_deref()
        && conditions.is_empty()
        && input.cursor.is_none()
    {
        let query = FileQuery {
            scope: &scope_filter,
            folder: folder.as_ref().map(PagePath::as_str),
            text,
            like_pattern,
            match_paths: matching == Match::TextAndPaths,
            limit,
        };
        found.extend(search_files(&mut conn, &query).await?);
    }
    let no_hits = if found.is_empty() && input.cursor.is_none() {
        Some(NoHits {
            query: describe(text, &conditions, folder.as_ref()),
            fields_in_scope: scope::field_names(&mut conn, &scope_filter).await?,
            folders_out_of_scope: u32::try_from(
                scope::hidden_folder_count(&mut conn, &scope_filter).await?,
            )
            .unwrap_or(u32::MAX),
        })
    } else {
        None
    };
    Ok(SearchOutput {
        hits: found,
        next_cursor,
        no_hits,
    })
}

fn describe(
    text: Option<&str>,
    conditions: &[ResolvedCondition],
    folder: Option<&PagePath>,
) -> String {
    let mut parts = Vec::new();
    if let Some(text) = text {
        parts.push(format!(
            "text \"{text}\" as a literal substring, case-sensitive"
        ));
    }
    parts.extend(
        conditions
            .iter()
            .map(|condition| format!("field {condition}")),
    );
    if let Some(folder) = folder {
        parts.push(format!("folder {folder} and below"));
    }
    if parts.is_empty() {
        return "every readable page".to_owned();
    }
    format!("pages with {}; all conditions must hold", parts.join("; "))
}
