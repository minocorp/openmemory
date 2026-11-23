use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use time::OffsetDateTime;
use utoipa::{OpenApi, ToSchema};

use crate::api::error::ApiError;
use crate::auth::ApiSession;
use crate::error::ErrorBody;
use crate::files::{FileId, FileRecord};
use crate::pages::ids::PageId;
use crate::pages::listing::PageSummary;
use crate::state::AppState;
use crate::tree::{self, Folder, Tree};

#[derive(Debug, Serialize, ToSchema)]
struct TreeView {
    folders: Vec<FolderView>,
    pages: Vec<TreePageView>,
    files: Vec<TreeFileView>,
}

#[derive(Debug, Serialize, ToSchema)]
struct FolderView {
    path: String,
    pages: u32,
    files: u32,
}

#[derive(Debug, Serialize, ToSchema)]
struct TreePageView {
    id: PageId,
    path: String,
    version: i32,
    #[serde(with = "time::serde::rfc3339")]
    updated_at: OffsetDateTime,
}

#[derive(Debug, Serialize, ToSchema)]
struct TreeFileView {
    id: FileId,
    path: String,
    mime: String,
    size: i64,
}

#[derive(OpenApi)]
#[openapi(
    paths(get_tree),
    components(schemas(TreeView, FolderView, TreePageView, TreeFileView))
)]
pub(super) struct Doc;

pub(super) fn router() -> Router<AppState> {
    Router::new().route("/api/tree", get(get_tree))
}

impl From<Tree> for TreeView {
    fn from(tree: Tree) -> Self {
        Self {
            folders: tree.folders.into_iter().map(FolderView::from).collect(),
            pages: tree.pages.into_iter().map(TreePageView::from).collect(),
            files: tree.files.into_iter().map(TreeFileView::from).collect(),
        }
    }
}

impl From<Folder> for FolderView {
    fn from(folder: Folder) -> Self {
        Self {
            path: folder.path,
            pages: folder.pages,
            files: folder.files,
        }
    }
}

impl From<PageSummary> for TreePageView {
    fn from(page: PageSummary) -> Self {
        Self {
            id: page.id,
            path: page.path,
            version: page.version.0,
            updated_at: page.updated_at,
        }
    }
}

impl From<FileRecord> for TreeFileView {
    fn from(file: FileRecord) -> Self {
        Self {
            id: file.id,
            path: file.path.to_string(),
            mime: file.mime,
            size: file.size,
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/tree",
    tag = "tree",
    security(("session" = []), ("bearer" = [])),
    responses((status = 200, body = TreeView), (status = 401, body = ErrorBody))
)]
async fn get_tree(
    State(state): State<AppState>,
    session: ApiSession,
) -> Result<Json<TreeView>, ApiError> {
    let tree = tree::load(state.pool(), state.files(), &session.actor).await?;
    Ok(Json(tree.into()))
}
