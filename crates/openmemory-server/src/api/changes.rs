use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use openmemory_api::{
    ChangeGroup, ChangedVersion, Changes, GroupReverted, Problem, SeenRequest, SeenState,
};
use serde::Deserialize;
use utoipa::OpenApi;

use crate::api::error::ApiError;
use crate::api::pages::{operation, written};
use crate::auth::ApiSession;
use crate::pages::changes::{self, self as found};
use crate::pages::ids::{GroupId, Seq};
use crate::pages::revert;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
struct ChangesQuery {
    since: Option<Seq>,
    cursor: Option<String>,
    limit: Option<u32>,
}

#[derive(OpenApi)]
#[openapi(
    paths(list_changes, mark_seen, revert_group),
    components(schemas(
        Changes,
        ChangeGroup,
        ChangedVersion,
        SeenState,
        SeenRequest,
        GroupReverted
    ))
)]
pub(super) struct Doc;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/changes", get(list_changes))
        .route("/api/changes/seen", post(mark_seen))
        .route("/api/changes/{group}/revert", post(revert_group))
}

fn changes_view(since: Seq, changes: found::Changes) -> Changes {
    Changes {
        since: since.0,
        last_seq: changes.last_seq.0,
        groups: changes.groups.into_iter().map(group_view).collect(),
        next: changes.next.map(|seq| seq.0.to_string()),
    }
}

fn group_view(group: found::ChangeGroup) -> ChangeGroup {
    ChangeGroup {
        id: group.id.0,
        task: group.task,
        author: group.author,
        author_display_name: group.author_display_name,
        at: group.at,
        versions: group.versions.into_iter().map(version_view).collect(),
    }
}

fn version_view(version: found::ChangedVersion) -> ChangedVersion {
    ChangedVersion {
        page_id: version.page_id.0,
        path: version.path,
        version: version.version,
        op: operation(version.op),
        seq: version.seq.0,
        diff: version.diff,
    }
}

#[utoipa::path(
    get,
    path = "/api/changes",
    tag = "changes",
    security(("session" = []), ("bearer" = [])),
    params(
        ("since" = Option<i64>, Query, description = "Defaults to the caller's last seen sequence"),
        ("cursor" = Option<String>, Query, description = "The `next` value of the previous page; overrides `since`"),
        ("limit" = Option<u32>, Query),
    ),
    responses((status = 200, body = Changes), (status = 401, body = Problem, content_type = "application/problem+json"))
)]
async fn list_changes(
    State(state): State<AppState>,
    session: ApiSession,
    Query(query): Query<ChangesQuery>,
) -> Result<Json<Changes>, ApiError> {
    let since = match (query.cursor.as_deref(), query.since) {
        (Some(cursor), _) => Seq(cursor
            .parse()
            .map_err(|_| ApiError::InvalidInput("cursor"))?),
        (None, Some(since)) => since,
        (None, None) => changes::last_seen(state.pool(), &session.actor).await?,
    };
    let found = changes::list(state.pool(), &session.actor, since, query.limit).await?;
    Ok(Json(changes_view(since, found)))
}

#[utoipa::path(
    post,
    path = "/api/changes/seen",
    tag = "changes",
    security(("session" = []), ("bearer" = [])),
    request_body = SeenRequest,
    responses((status = 200, body = SeenState), (status = 401, body = Problem, content_type = "application/problem+json"))
)]
async fn mark_seen(
    State(state): State<AppState>,
    session: ApiSession,
    Json(request): Json<SeenRequest>,
) -> Result<Json<SeenState>, ApiError> {
    let last_seen_seq = changes::mark_seen(state.pool(), &session.actor, Seq(request.seq)).await?;
    Ok(Json(SeenState {
        last_seen_seq: last_seen_seq.0,
    }))
}

#[utoipa::path(
    post,
    path = "/api/changes/{group}/revert",
    tag = "changes",
    security(("session" = []), ("bearer" = [])),
    params(("group" = i64, Path)),
    responses(
        (status = 200, body = GroupReverted),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 404, body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "A page changed since the group or its path is taken; nothing was reverted", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn revert_group(
    State(state): State<AppState>,
    session: ApiSession,
    Path(group): Path<GroupId>,
) -> Result<Json<GroupReverted>, ApiError> {
    let output = revert::revert_group(state.pool(), &session.actor, group, None).await?;
    Ok(Json(GroupReverted {
        reverted: output.reverted.into_iter().map(written).collect(),
    }))
}
