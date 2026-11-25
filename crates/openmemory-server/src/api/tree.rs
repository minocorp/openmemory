use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use openmemory_api::{Problem, TreeFile, TreeFolder, TreePage, TreeView};
use utoipa::OpenApi;

use crate::api::error::ApiError;
use crate::auth::ApiSession;
use crate::files::FileRecord;
use crate::pages::listing::PageSummary;
use crate::state::AppState;
use crate::tree::{self, Folder, Tree};

#[derive(OpenApi)]
#[openapi(
    paths(get_tree),
    components(schemas(TreeView, TreeFolder, TreePage, TreeFile))
)]
pub(super) struct Doc;

pub(super) fn router() -> Router<AppState> {
    Router::new().route("/api/tree", get(get_tree))
}

fn tree_view(tree: Tree) -> TreeView {
    TreeView {
        folders: tree.folders.into_iter().map(tree_folder).collect(),
        pages: tree.pages.into_iter().map(tree_page).collect(),
        files: tree.files.into_iter().map(tree_file).collect(),
    }
}

fn tree_folder(folder: Folder) -> TreeFolder {
    TreeFolder {
        path: folder.path,
        pages: folder.pages,
        files: folder.files,
    }
}

fn tree_page(page: PageSummary) -> TreePage {
    TreePage {
        id: page.id.0,
        path: page.path,
        version: page.version.0,
        updated_at: page.updated_at,
    }
}

fn tree_file(file: FileRecord) -> TreeFile {
    TreeFile {
        id: file.id.0,
        path: file.path.to_string(),
        mime: file.mime,
        size: file.size,
    }
}

#[utoipa::path(
    get,
    path = "/api/tree",
    tag = "tree",
    security(("session" = []), ("bearer" = [])),
    responses((status = 200, body = TreeView), (status = 401, body = Problem, content_type = "application/problem+json"))
)]
async fn get_tree(
    State(state): State<AppState>,
    session: ApiSession,
) -> Result<Json<TreeView>, ApiError> {
    let tree = tree::load(state.pool(), state.files(), &session.actor).await?;
    Ok(Json(tree_view(tree)))
}
