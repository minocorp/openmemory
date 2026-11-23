use std::collections::HashMap;

use axum::extract::{RawQuery, State};
use axum::routing::get;
use axum::{Json, Router};
use memory_ops::{HitKind, NoHits, SearchHit, SearchInput};
use serde::Serialize;
use utoipa::{OpenApi, ToSchema};

use crate::api::error::ApiError;
use crate::auth::ApiSession;
use crate::error::ErrorBody;
use crate::files::FileId;
use crate::pages::ids::PageId;
use crate::pages::listing;
use crate::search::{self, Match};
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
struct SearchView {
    pages: Vec<PageHitView>,
    files: Vec<FileHitView>,
    next_cursor: Option<i64>,
    no_hits: Option<NoHitsView>,
}

#[derive(Debug, Serialize, ToSchema)]
struct PageHitView {
    page_id: PageId,
    path: String,
    line_number: Option<u32>,
    line: Option<String>,
    author: String,
    at: String,
}

#[derive(Debug, Serialize, ToSchema)]
struct FileHitView {
    file_id: FileId,
    path: String,
    page: Option<u32>,
    line_number: Option<u32>,
    line: Option<String>,
    author: String,
    at: String,
}

#[derive(Debug, Serialize, ToSchema)]
struct NoHitsView {
    query: String,
    fields_in_scope: Vec<String>,
    folders_out_of_scope: u32,
}

#[derive(OpenApi)]
#[openapi(
    paths(search),
    components(schemas(SearchView, PageHitView, FileHitView, NoHitsView))
)]
pub(super) struct Doc;

pub(super) fn router() -> Router<AppState> {
    Router::new().route("/api/search", get(search))
}

impl From<NoHits> for NoHitsView {
    fn from(no_hits: NoHits) -> Self {
        Self {
            query: no_hits.query,
            fields_in_scope: no_hits.fields_in_scope,
            folders_out_of_scope: no_hits.folders_out_of_scope,
        }
    }
}

fn parse_params(raw: &str) -> Result<SearchInput, ApiError> {
    let mut input = SearchInput::default();
    for (key, value) in url::form_urlencoded::parse(raw.as_bytes()) {
        match key.as_ref() {
            "q" => input.text = Some(value.into_owned()),
            "where" => input.conditions.push(value.into_owned()),
            "folder" => input.folder = Some(value.into_owned()),
            "limit" => {
                input.limit = Some(value.parse().map_err(|_| ApiError::InvalidInput("limit"))?);
            }
            "cursor" => {
                input.cursor = Some(
                    value
                        .parse()
                        .map_err(|_| ApiError::InvalidInput("cursor"))?,
                );
            }
            _ => {}
        }
    }
    Ok(input)
}

#[utoipa::path(
    get,
    path = "/api/search",
    tag = "search",
    security(("session" = []), ("bearer" = [])),
    params(
        ("q" = Option<String>, Query, description = "Literal text"),
        ("where" = Option<Vec<String>>, Query, style = Form, explode = true, description = "Field conditions, ANDed; repeat the parameter"),
        ("folder" = Option<String>, Query),
        ("limit" = Option<u32>, Query),
        ("cursor" = Option<i64>, Query),
    ),
    responses(
        (status = 200, body = SearchView),
        (status = 400, body = ErrorBody),
        (status = 401, body = ErrorBody),
    )
)]
async fn search(
    State(state): State<AppState>,
    session: ApiSession,
    RawQuery(raw): RawQuery,
) -> Result<Json<SearchView>, ApiError> {
    let input = parse_params(raw.as_deref().unwrap_or_default())?;
    let output = search::search(
        state.pool(),
        state.time_zone(),
        &session.actor,
        &input,
        Match::TextAndPaths,
    )
    .await?;
    let (file_hits, page_hits): (Vec<_>, Vec<_>) = output
        .hits
        .into_iter()
        .partition(|hit| hit.kind == HitKind::File);
    let page_ids: HashMap<String, PageId> =
        listing::ids_for_paths(state.pool(), &paths_of(&page_hits))
            .await?
            .into_iter()
            .collect();
    let file_ids: HashMap<String, FileId> = state
        .files()
        .ids_for_paths(&paths_of(&file_hits))
        .await?
        .into_iter()
        .collect();
    Ok(Json(SearchView {
        pages: page_hits
            .into_iter()
            .filter_map(|hit| page_hit(&page_ids, hit))
            .collect(),
        files: file_hits
            .into_iter()
            .filter_map(|hit| file_hit(&file_ids, hit))
            .collect(),
        next_cursor: output.next_cursor,
        no_hits: output.no_hits.map(NoHitsView::from),
    }))
}

fn paths_of(hits: &[SearchHit]) -> Vec<String> {
    hits.iter().map(|hit| hit.path.clone()).collect()
}

fn page_hit(ids: &HashMap<String, PageId>, hit: SearchHit) -> Option<PageHitView> {
    Some(PageHitView {
        page_id: *ids.get(&hit.path)?,
        path: hit.path,
        line_number: hit.line_number,
        line: hit.line,
        author: hit.author,
        at: hit.at,
    })
}

fn file_hit(ids: &HashMap<String, FileId>, hit: SearchHit) -> Option<FileHitView> {
    Some(FileHitView {
        file_id: *ids.get(&hit.path)?,
        path: hit.path,
        page: hit.page,
        line_number: hit.line_number,
        line: hit.line,
        author: hit.author,
        at: hit.at,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_where_parameters_become_separate_conditions() {
        let input = parse_params(
            "q=fee&where=status%3Dactive&where=renewal%3Etoday&folder=clients&limit=5",
        )
        .unwrap();
        assert_eq!(input.text.as_deref(), Some("fee"));
        assert_eq!(input.conditions, ["status=active", "renewal>today"]);
        assert_eq!(input.folder.as_deref(), Some("clients"));
        assert_eq!(input.limit, Some(5));
    }

    #[test]
    fn numeric_parameters_must_parse() {
        assert!(matches!(
            parse_params("limit=many"),
            Err(ApiError::InvalidInput("limit"))
        ));
        assert!(matches!(
            parse_params("cursor=x"),
            Err(ApiError::InvalidInput("cursor"))
        ));
    }

    #[test]
    fn unknown_parameters_are_ignored() {
        assert_eq!(parse_params("other=1").unwrap(), SearchInput::default());
    }
}
