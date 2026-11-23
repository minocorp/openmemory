use std::collections::HashMap;

use axum::extract::{RawQuery, State};
use axum::routing::get;
use axum::{Json, Router};
use memory_api::{FileHit, NoHits as ApiNoHits, PageHit, Problem, SearchResults};
use memory_ops::{HitKind, NoHits, SearchHit, SearchInput};
use utoipa::OpenApi;

use crate::api::error::ApiError;
use crate::auth::WebSession;
use crate::files::FileId;
use crate::pages::ids::PageId;
use crate::pages::listing;
use crate::search::{self, Match};
use crate::state::AppState;

#[derive(OpenApi)]
#[openapi(
    paths(search),
    components(schemas(SearchResults, PageHit, FileHit, ApiNoHits))
)]
pub(super) struct Doc;

pub(super) fn router() -> Router<AppState> {
    Router::new().route("/api/search", get(search))
}

fn no_hits_view(no_hits: NoHits) -> ApiNoHits {
    ApiNoHits {
        query: no_hits.query,
        fields_in_scope: no_hits.fields_in_scope,
        folders_out_of_scope: no_hits.folders_out_of_scope,
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
    security(("session" = [])),
    params(
        ("q" = Option<String>, Query, description = "Literal text"),
        ("where" = Option<Vec<String>>, Query, style = Form, explode = true, description = "Field conditions, ANDed; repeat the parameter"),
        ("folder" = Option<String>, Query),
        ("limit" = Option<u32>, Query),
        ("cursor" = Option<String>, Query, description = "The `next` value of the previous page"),
    ),
    responses(
        (status = 200, body = SearchResults),
        (status = 400, body = Problem, content_type = "application/problem+json"),
        (status = 401, body = Problem, content_type = "application/problem+json"),
    )
)]
async fn search(
    State(state): State<AppState>,
    session: WebSession,
    RawQuery(raw): RawQuery,
) -> Result<Json<SearchResults>, ApiError> {
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
    Ok(Json(SearchResults {
        pages: page_hits
            .into_iter()
            .filter_map(|hit| page_hit(&page_ids, hit))
            .collect(),
        files: file_hits
            .into_iter()
            .filter_map(|hit| file_hit(&file_ids, hit))
            .collect(),
        next: output.next_cursor.map(|cursor| cursor.to_string()),
        no_hits: output.no_hits.map(no_hits_view),
    }))
}

fn paths_of(hits: &[SearchHit]) -> Vec<String> {
    hits.iter().map(|hit| hit.path.clone()).collect()
}

fn page_hit(ids: &HashMap<String, PageId>, hit: SearchHit) -> Option<PageHit> {
    Some(PageHit {
        page_id: ids.get(&hit.path)?.0,
        path: hit.path,
        line_number: hit.line_number,
        line: hit.line,
        author: hit.author,
        at: hit.at,
    })
}

fn file_hit(ids: &HashMap<String, FileId>, hit: SearchHit) -> Option<FileHit> {
    Some(FileHit {
        file_id: ids.get(&hit.path)?.0,
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
