use axum::extract::{RawQuery, State};
use axum::routing::get;
use axum::{Json, Router};
use openmemory_api::{FolderHits, HitKind, NoHits as ApiNoHits, Problem, SearchHit, SearchResults};
use openmemory_ops::{NoHits, SearchInput};
use utoipa::OpenApi;

use crate::api::error::ApiError;
use crate::auth::ApiSession;
use crate::search::{self, Document, Found, FoundFolder, FoundHit};
use crate::state::AppState;

#[derive(OpenApi)]
#[openapi(
    paths(search),
    components(schemas(SearchResults, SearchHit, HitKind, FolderHits, ApiNoHits))
)]
pub(super) struct Doc;

pub(super) fn router() -> Router<AppState> {
    Router::new().route("/api/search", get(search))
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
            "cursor" => input.cursor = Some(value.into_owned()),
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
        ("q" = Option<String>, Query, description = "Words; each must occur in the path or text, ignoring case"),
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
    session: ApiSession,
    RawQuery(raw): RawQuery,
) -> Result<Json<SearchResults>, ApiError> {
    let input = parse_params(raw.as_deref().unwrap_or_default())?;
    let found = search::search(state.pool(), state.time_zone(), &session.actor, &input).await?;
    Ok(Json(found.into()))
}

impl From<Found> for SearchResults {
    fn from(found: Found) -> Self {
        Self {
            hits: found.hits.into_iter().map(Into::into).collect(),
            folders: found.folders.into_iter().map(Into::into).collect(),
            next: found.next_cursor,
            no_hits: found.no_hits.map(no_hits_view),
        }
    }
}

impl From<FoundFolder> for FolderHits {
    fn from(folder: FoundFolder) -> Self {
        Self {
            path: folder.path,
            matches: folder.matches,
            hits: folder.hits.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<FoundHit> for SearchHit {
    fn from(found: FoundHit) -> Self {
        let (kind, id) = match found.document {
            Document::Page(id) => (HitKind::Page, id.0),
            Document::File(id) => (HitKind::File, id.0),
        };
        Self {
            kind,
            id,
            path: found.hit.path,
            page: found.hit.page,
            line_number: found.hit.line_number,
            line: found.hit.line,
            author: found.hit.author,
            at: found.hit.at,
        }
    }
}

fn no_hits_view(no_hits: NoHits) -> ApiNoHits {
    ApiNoHits {
        query: no_hits.query,
        fields_in_scope: no_hits.fields_in_scope,
        folders_out_of_scope: no_hits.folders_out_of_scope,
    }
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
    fn the_limit_must_be_a_number() {
        assert!(matches!(
            parse_params("limit=many"),
            Err(ApiError::InvalidInput("limit"))
        ));
    }

    #[test]
    fn unknown_parameters_are_ignored() {
        assert_eq!(parse_params("other=1").unwrap(), SearchInput::default());
    }
}
