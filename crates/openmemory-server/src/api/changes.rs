use std::time::Duration;

use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::response::IntoResponse;
use axum::response::sse::{KeepAlive, Sse};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::StreamExt;
use openmemory_api::{
    ChangeEvent, ChangeGroup, ChangedVersion, Changes, FileOperation, GroupReverted, Problem,
    PurgedKind, SeenRequest, SeenState, StreamReady,
};
use serde::Deserialize;
use utoipa::OpenApi;

use crate::api::error::ApiError;
use crate::api::pages::{operation, written};
use crate::auth::ApiSession;
use crate::journal::{Change, Cursor, Event, FileOp};
use crate::pages::changes::{self, self as found};
use crate::pages::ids::{GroupId, Seq};
use crate::pages::revert;
use crate::purge::Subject;
use crate::state::AppState;

const KEEP_ALIVE: Duration = Duration::from_secs(20);
// A stream outlives no session check by more than this; the client reconnects and is authorized again.
const STREAM_LIFETIME: Duration = Duration::from_mins(5);

#[derive(Debug, Deserialize)]
struct ChangesQuery {
    since: Option<Seq>,
    cursor: Option<String>,
    limit: Option<u32>,
}

#[derive(OpenApi)]
#[openapi(
    paths(list_changes, mark_seen, revert_group, stream_changes),
    components(schemas(
        Changes,
        ChangeGroup,
        ChangedVersion,
        SeenState,
        SeenRequest,
        GroupReverted,
        StreamReady,
        ChangeEvent,
        FileOperation,
        PurgedKind
    ))
)]
pub(super) struct Doc;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/changes", get(list_changes))
        .route("/api/changes/seen", post(mark_seen))
        .route("/api/changes/{group}/revert", post(revert_group))
}

pub(super) fn streams() -> Router<AppState> {
    Router::new().route("/api/changes/stream", get(stream_changes))
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

#[utoipa::path(
    get,
    path = "/api/changes/stream",
    tag = "changes",
    security(("session" = []), ("bearer" = [])),
    params(("Last-Event-ID" = Option<String>, Header, description = "The id of the last event received; the stream resumes after it")),
    responses(
        (status = 200, description = "Server-sent events. `ready` (StreamReady) comes first; when `resumed` is false, everything read before may be stale. Then `change` (ChangeEvent), in commit order. Every id resumes the stream.", content_type = "text/event-stream"),
        (status = 401, body = Problem, content_type = "application/problem+json"),
    )
)]
async fn stream_changes(
    State(state): State<AppState>,
    session: ApiSession,
    headers: HeaderMap,
) -> impl IntoResponse {
    let from = headers
        .get("last-event-id")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<Cursor>().ok());
    let events = state
        .journal()
        .follow(session.actor.scope, from)
        .take_until(tokio::time::sleep(STREAM_LIFETIME))
        .map(sse_event);
    (
        [("x-accel-buffering", "no")],
        Sse::new(events).keep_alive(KeepAlive::new().interval(KEEP_ALIVE)),
    )
}

fn sse_event(event: Event) -> Result<axum::response::sse::Event, axum::Error> {
    let sse = axum::response::sse::Event::default();
    match event {
        Event::Ready { cursor, resumed } => sse
            .event("ready")
            .id(cursor.to_string())
            .json_data(StreamReady { resumed }),
        Event::Change { cursor, change } => sse
            .event("change")
            .id(cursor.to_string())
            .json_data(change_view(change)),
        Event::Passed { cursor } => Ok(sse.id(cursor.to_string())),
    }
}

fn change_view(change: Change) -> ChangeEvent {
    match change {
        Change::Page {
            id,
            version,
            op,
            path,
            previous_path,
        } => ChangeEvent::Page {
            id: id.0,
            version: version.0,
            op: operation(op),
            path: path.map(|path| path.as_str().to_owned()),
            previous_path: previous_path.map(|path| path.as_str().to_owned()),
        },
        Change::File { id, op, path } => ChangeEvent::File {
            id: id.0,
            op: match op {
                FileOp::Upload => FileOperation::Upload,
                FileOp::Delete => FileOperation::Delete,
            },
            path: path.as_str().to_owned(),
        },
        Change::Purge { subject, id, path } => ChangeEvent::Purge {
            subject: match subject {
                Subject::Page => PurgedKind::Page,
                Subject::File => PurgedKind::File,
            },
            id,
            path: path.as_str().to_owned(),
        },
    }
}
