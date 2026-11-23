use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};

use crate::api::error::ApiError;
use crate::api::pages::{OperationView, WrittenView};
use crate::auth::ApiSession;
use crate::error::ErrorBody;
use crate::pages::changes::{self, ChangeGroup, ChangedVersion, Changes};
use crate::pages::ids::{GroupId, PageId, Seq};
use crate::pages::revert;
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
struct ChangesView {
    since: Seq,
    last_seq: Seq,
    groups: Vec<ChangeGroupView>,
}

#[derive(Debug, Serialize, ToSchema)]
struct ChangeGroupView {
    id: GroupId,
    task: Option<String>,
    author: String,
    author_display_name: String,
    at: String,
    versions: Vec<ChangedVersionView>,
}

#[derive(Debug, Serialize, ToSchema)]
struct ChangedVersionView {
    page_id: PageId,
    path: String,
    version: i32,
    op: OperationView,
    seq: Seq,
    diff: String,
}

#[derive(Debug, Serialize, ToSchema)]
struct SeenView {
    last_seen_seq: Seq,
}

#[derive(Debug, Deserialize, ToSchema)]
struct SeenRequest {
    seq: Seq,
}

#[derive(Debug, Serialize, ToSchema)]
struct GroupRevertedView {
    reverted: Vec<WrittenView>,
}

#[derive(Debug, Deserialize)]
struct ChangesQuery {
    since: Option<Seq>,
    limit: Option<u32>,
}

#[derive(OpenApi)]
#[openapi(
    paths(list_changes, mark_seen, revert_group),
    components(schemas(
        ChangesView,
        ChangeGroupView,
        ChangedVersionView,
        SeenView,
        SeenRequest,
        GroupRevertedView
    ))
)]
pub(super) struct Doc;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/changes", get(list_changes))
        .route("/api/changes/seen", post(mark_seen))
        .route("/api/changes/{group}/revert", post(revert_group))
}

impl ChangesView {
    fn new(since: Seq, changes: Changes) -> Self {
        Self {
            since,
            last_seq: changes.last_seq,
            groups: changes
                .groups
                .into_iter()
                .map(ChangeGroupView::from)
                .collect(),
        }
    }
}

impl From<ChangeGroup> for ChangeGroupView {
    fn from(group: ChangeGroup) -> Self {
        Self {
            id: group.id,
            task: group.task,
            author: group.author,
            author_display_name: group.author_display_name,
            at: group.at,
            versions: group
                .versions
                .into_iter()
                .map(ChangedVersionView::from)
                .collect(),
        }
    }
}

impl From<ChangedVersion> for ChangedVersionView {
    fn from(version: ChangedVersion) -> Self {
        Self {
            page_id: version.page_id,
            path: version.path,
            version: version.version,
            op: version.op.into(),
            seq: version.seq,
            diff: version.diff,
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/changes",
    tag = "changes",
    security(("session" = []), ("bearer" = [])),
    params(
        ("since" = Option<i64>, Query, description = "Defaults to the caller's last seen sequence"),
        ("limit" = Option<u32>, Query),
    ),
    responses((status = 200, body = ChangesView), (status = 401, body = ErrorBody))
)]
async fn list_changes(
    State(state): State<AppState>,
    session: ApiSession,
    Query(query): Query<ChangesQuery>,
) -> Result<Json<ChangesView>, ApiError> {
    let since = match query.since {
        Some(since) => since,
        None => changes::last_seen(state.pool(), &session.actor).await?,
    };
    let found = changes::list(state.pool(), &session.actor, since, query.limit).await?;
    Ok(Json(ChangesView::new(since, found)))
}

#[utoipa::path(
    post,
    path = "/api/changes/seen",
    tag = "changes",
    security(("session" = []), ("bearer" = [])),
    request_body = SeenRequest,
    responses((status = 200, body = SeenView), (status = 401, body = ErrorBody))
)]
async fn mark_seen(
    State(state): State<AppState>,
    session: ApiSession,
    Json(request): Json<SeenRequest>,
) -> Result<Json<SeenView>, ApiError> {
    let last_seen_seq = changes::mark_seen(state.pool(), &session.actor, request.seq).await?;
    Ok(Json(SeenView { last_seen_seq }))
}

#[utoipa::path(
    post,
    path = "/api/changes/{group}/revert",
    tag = "changes",
    security(("session" = []), ("bearer" = [])),
    params(("group" = i64, Path)),
    responses(
        (status = 200, body = GroupRevertedView),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 409, description = "A page changed since the group or its path is taken; nothing was reverted", body = ErrorBody),
    )
)]
async fn revert_group(
    State(state): State<AppState>,
    session: ApiSession,
    Path(group): Path<GroupId>,
) -> Result<Json<GroupRevertedView>, ApiError> {
    let output = revert::revert_group(state.pool(), &session.actor, group, None).await?;
    Ok(Json(GroupRevertedView {
        reverted: output.reverted.into_iter().map(WrittenView::from).collect(),
    }))
}
