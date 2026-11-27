mod candidate;
mod error;
mod hits;
mod layout;
mod position;
mod queries;
mod terms;

#[cfg(test)]
mod tests;

use openmemory_ops::{
    FolderHits, NoHits, PagePath, ResolvedCondition, SearchInput, SearchOutput, WhereClause,
};
use std::cmp::Reverse;

use sqlx::{PgConnection, PgPool};

use crate::access::Actor;
use crate::pages::scope_filter::ScopeFilter;
use crate::search::candidate::Candidate;
use crate::search::hits::Texts;
use crate::search::layout::Layout;
use crate::search::position::Position;
use crate::search::queries::matches::{self, MatchFilter};
use crate::search::queries::{scope, texts};
use crate::search::terms::Terms;
use crate::time_zone::TimeZone;

pub(crate) use error::SearchError;
pub(crate) use hits::FoundHit;
pub(crate) use position::Document;

const DEFAULT_LIMIT: u32 = 20;
const MAX_LIMIT: u32 = 100;
const LINES_PER_HIT: usize = 5;
const LINES_PER_PREVIEW: usize = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Found {
    pub(crate) hits: Vec<FoundHit>,
    pub(crate) folders: Vec<FoundFolder>,
    pub(crate) next_cursor: Option<String>,
    pub(crate) no_hits: Option<NoHits>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FoundFolder {
    pub(crate) path: String,
    pub(crate) matches: u32,
    pub(crate) hits: Vec<FoundHit>,
}

pub(crate) async fn search(
    pool: &PgPool,
    time_zone: &TimeZone,
    actor: &Actor,
    input: &SearchInput,
) -> Result<Found, SearchError> {
    let mut conn = pool.acquire().await?;
    let today = time_zone.today(&mut conn).await?;
    let conditions = WhereClause::parse(&input.conditions)?.resolve(today)?;
    let folder = input
        .folder
        .as_deref()
        .map(PagePath::parse)
        .transpose()
        .map_err(SearchError::Folder)?;
    let after = input
        .cursor
        .as_deref()
        .map(str::parse::<Position>)
        .transpose()
        .map_err(|_| SearchError::Cursor)?;
    let terms = Terms::parse(input.text.as_deref());
    let like_patterns = terms.like_patterns();
    let scope_filter = ScopeFilter::of(&actor.scope);
    let filter = MatchFilter {
        scope: &scope_filter,
        folder: folder.as_ref().map(PagePath::as_str),
        like_patterns: &like_patterns,
        conditions: &conditions,
    };
    let candidates = find_candidates(&mut conn, &filter).await?;
    if candidates.is_empty() {
        let no_hits = match after {
            Some(_) => None,
            None => Some(explain(&mut conn, &filter, &terms, folder.as_ref()).await?),
        };
        return Ok(Found {
            hits: Vec::new(),
            folders: Vec::new(),
            next_cursor: None,
            no_hits,
        });
    }
    let limit = usize::try_from(input.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT))
        .unwrap_or(usize::MAX);
    let layout = layout::lay_out(
        &candidates,
        folder.as_ref().map_or("", PagePath::as_str),
        after,
        limit,
    );
    let texts = load_texts(&mut conn, &layout, &like_patterns).await?;
    Ok(assemble(&layout, &texts, &terms))
}

async fn find_candidates(
    conn: &mut PgConnection,
    filter: &MatchFilter<'_>,
) -> Result<Vec<Candidate>, SearchError> {
    let mut candidates = matches::pages(conn, filter).await?;
    if filter.conditions.is_empty() {
        candidates.extend(matches::files(conn, filter).await?);
    }
    candidates.sort_unstable_by_key(|candidate| Reverse(candidate.position()));
    Ok(candidates)
}

fn assemble(layout: &Layout<'_>, texts: &Texts, terms: &Terms) -> Found {
    Found {
        hits: layout
            .hits
            .iter()
            .flat_map(|candidate| texts.hits_for(candidate, terms, LINES_PER_HIT))
            .collect(),
        folders: layout
            .folders
            .iter()
            .map(|group| FoundFolder {
                path: group.path.clone(),
                matches: u32::try_from(group.matches).unwrap_or(u32::MAX),
                hits: group
                    .preview
                    .iter()
                    .flat_map(|candidate| texts.hits_for(candidate, terms, LINES_PER_PREVIEW))
                    .collect(),
            })
            .collect(),
        next_cursor: layout.next.map(|position| position.to_string()),
        no_hits: None,
    }
}

async fn load_texts(
    conn: &mut PgConnection,
    layout: &Layout<'_>,
    like_patterns: &[String],
) -> Result<Texts, SearchError> {
    let shown = layout
        .hits
        .iter()
        .chain(layout.folders.iter().flat_map(|group| group.preview.iter()));
    let mut page_ids = Vec::new();
    let mut file_ids = Vec::new();
    for candidate in shown {
        match candidate.document {
            Document::Page(id) => page_ids.push(id.0),
            Document::File(id) => file_ids.push(id.0),
        }
    }
    let pages = texts::pages(conn, &page_ids).await?;
    let files = if file_ids.is_empty() || like_patterns.is_empty() {
        Vec::new()
    } else {
        texts::first_matching_file_pages(conn, &file_ids, like_patterns).await?
    };
    Ok(Texts::new(pages, files))
}

async fn explain(
    conn: &mut PgConnection,
    filter: &MatchFilter<'_>,
    terms: &Terms,
    folder: Option<&PagePath>,
) -> Result<NoHits, SearchError> {
    Ok(NoHits {
        query: describe(terms, filter.conditions, folder),
        fields_in_scope: scope::field_names(conn, filter.scope).await?,
        folders_out_of_scope: u32::try_from(scope::hidden_folder_count(conn, filter.scope).await?)
            .unwrap_or(u32::MAX),
    })
}

fn describe(terms: &Terms, conditions: &[ResolvedCondition], folder: Option<&PagePath>) -> String {
    let mut parts = Vec::new();
    if !terms.is_empty() {
        let words = terms
            .words()
            .iter()
            .map(|word| format!("\"{word}\""))
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(format!("words {words} in the path or text, ignoring case"));
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
        return "every readable page and file".to_owned();
    }
    format!("{}; all must hold", parts.join("; "))
}

impl From<FoundHit> for openmemory_ops::SearchHit {
    fn from(found: FoundHit) -> Self {
        found.hit
    }
}

impl From<Found> for SearchOutput {
    fn from(found: Found) -> Self {
        Self {
            hits: found.hits.into_iter().map(Into::into).collect(),
            folders: found
                .folders
                .into_iter()
                .map(|folder| FolderHits {
                    path: folder.path,
                    matches: folder.matches,
                    hits: folder.hits.into_iter().map(Into::into).collect(),
                })
                .collect(),
            next_cursor: found.next_cursor,
            no_hits: found.no_hits,
        }
    }
}
