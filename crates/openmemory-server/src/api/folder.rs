use axum::extract::{Query, State};
use axum::routing::get;
use axum::{Json, Router};
use openmemory_api::{EntryKind, FolderEntry, FolderListing, Problem};
use openmemory_ops::PagePath;
use serde::Deserialize;
use utoipa::OpenApi;

use crate::api::error::ApiError;
use crate::auth::ApiSession;
use crate::pages::error::PagesError;
use crate::pages::folder;
use crate::state::AppState;

const DEFAULT_LIMIT: usize = 100;
const MAX_LIMIT: usize = 500;

#[derive(Debug, Deserialize)]
struct FolderQuery {
    path: Option<String>,
    limit: Option<usize>,
    cursor: Option<String>,
}

#[derive(OpenApi)]
#[openapi(
    paths(list_folder),
    components(schemas(FolderListing, FolderEntry, EntryKind))
)]
pub(super) struct Doc;

pub(super) fn router() -> Router<AppState> {
    Router::new().route("/api/folder", get(list_folder))
}

fn entry_kind(kind: openmemory_ops::EntryKind) -> EntryKind {
    match kind {
        openmemory_ops::EntryKind::Page => EntryKind::Page,
        openmemory_ops::EntryKind::File => EntryKind::File,
        openmemory_ops::EntryKind::Folder => EntryKind::Folder,
    }
}

#[utoipa::path(
    get,
    path = "/api/folder",
    tag = "tree",
    security(("session" = []), ("bearer" = [])),
    params(
        ("path" = Option<String>, Query, description = "Folder path; empty for the top level"),
        ("limit" = Option<u32>, Query, description = "At most 500; default 100"),
        ("cursor" = Option<String>, Query, description = "The `next` value of the previous page"),
    ),
    responses(
        (status = 200, body = FolderListing),
        (status = 400, body = Problem, content_type = "application/problem+json"),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "Nothing at the path", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn list_folder(
    State(state): State<AppState>,
    session: ApiSession,
    Query(query): Query<FolderQuery>,
) -> Result<Json<FolderListing>, ApiError> {
    let path = query
        .path
        .as_deref()
        .map(|raw| raw.trim_matches('/'))
        .filter(|raw| !raw.is_empty())
        .map(PagePath::parse)
        .transpose()
        .map_err(|_| ApiError::InvalidInput("path"))?;
    let offset = query
        .cursor
        .as_deref()
        .map(str::parse::<usize>)
        .transpose()
        .map_err(|_| ApiError::InvalidInput("cursor"))?
        .unwrap_or(0);
    let limit = query.limit.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_LIMIT);
    let mut entries = folder::entries(state.pool(), &session.actor, path.as_ref()).await?;
    for entry in state.files().entries(&session.actor, path.as_ref()).await? {
        if !entries.contains(&entry) {
            entries.push(entry);
        }
    }
    if entries.is_empty() && path.is_some() {
        return Err(PagesError::NotFound.into());
    }
    entries.sort_by(|left, right| left.name.cmp(&right.name));
    let end = offset.saturating_add(limit).min(entries.len());
    let next = (end < entries.len()).then(|| end.to_string());
    let page = entries
        .get(offset.min(end)..end)
        .unwrap_or_default()
        .iter()
        .map(|entry| FolderEntry {
            name: entry.name.clone(),
            kind: entry_kind(entry.kind),
        })
        .collect();
    Ok(Json(FolderListing {
        path: path.map_or_else(|| "/".to_owned(), |path| path.to_string()),
        entries: page,
        next,
    }))
}
