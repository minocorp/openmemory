use axum::extract::{Path, Query, State};
use axum::http::header::{ETAG, LOCATION};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use openmemory_api::{
    AppendBody, FieldView, FileSourceView, Moved, Operation, PageCreate, PageEdit, PageMove,
    PageOverwrite, PageView, Problem, ReplaceBody, Restored, TimelineView, VersionList,
    VersionView, Written,
};
use openmemory_ops::{
    Append, FieldRead, FileSourceRead, PagePath, PageRead, Replacement, TimelineRead, VersionEntry,
    VersionOp, WriteAction, WriteOutput, parse_iso_date,
};
use serde::{Deserialize, Serialize};
use utoipa::OpenApi;

use crate::api::error::ApiError;
use crate::api::precondition::{etag, if_match, matches_none_match, requires_absent};
use crate::auth::ApiSession;
use crate::pages::error::PagesError;
use crate::pages::ids::{PageId, VersionNumber};
use crate::pages::write::Revision;
use crate::pages::{delete, history, locate, read, relocate, revert, write};
use crate::state::AppState;

#[derive(Debug, Deserialize)]
struct PathQuery {
    path: String,
}

#[derive(Debug, Deserialize)]
struct VersionQuery {
    version: Option<i32>,
}

#[derive(Debug, Deserialize)]
struct VersionsQuery {
    limit: Option<u32>,
    cursor: Option<String>,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        get_page_by_path,
        create_page,
        get_page,
        overwrite_page,
        edit_page,
        delete_page,
        list_versions,
        restore_version,
        move_page
    ),
    components(schemas(
        PageView,
        FieldView,
        TimelineView,
        FileSourceView,
        Operation,
        VersionView,
        VersionList,
        Written,
        Moved,
        Restored,
        PageCreate,
        PageOverwrite,
        ReplaceBody,
        AppendBody,
        PageEdit,
        PageMove
    ))
)]
pub(super) struct Doc;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/pages", get(get_page_by_path).put(create_page))
        .route(
            "/api/pages/{id}",
            get(get_page)
                .put(overwrite_page)
                .patch(edit_page)
                .delete(delete_page),
        )
        .route("/api/pages/{id}/versions", get(list_versions))
        .route(
            "/api/pages/{id}/versions/{n}/restore",
            post(restore_version),
        )
        .route("/api/pages/{id}/move", post(move_page))
}

pub(super) fn page_view(id: PageId, page: PageRead) -> PageView {
    PageView {
        id: id.0,
        path: page.path,
        version: page.version,
        author: page.author,
        updated_at: page.updated_at,
        fields: page.fields.into_iter().map(field_view).collect(),
        body: page.body,
        timeline: page.timeline.into_iter().map(timeline_view).collect(),
        content: page.content,
    }
}

fn field_view(field: FieldRead) -> FieldView {
    FieldView {
        key: field.key,
        values: field.values,
    }
}

fn timeline_view(entry: TimelineRead) -> TimelineView {
    TimelineView {
        date: entry.date,
        source: entry.source,
        file: entry.file.map(file_source_view),
        text: entry.text,
    }
}

fn file_source_view(source: FileSourceRead) -> FileSourceView {
    FileSourceView {
        path: source.path,
        page: source.page,
        quote: source.quote,
    }
}

pub(super) fn operation(op: VersionOp) -> Operation {
    match op {
        VersionOp::Create => Operation::Create,
        VersionOp::Edit => Operation::Edit,
        VersionOp::Move => Operation::Move,
        VersionOp::Relink => Operation::Relink,
        VersionOp::Delete => Operation::Delete,
        VersionOp::Revert => Operation::Revert,
    }
}

fn version_view(entry: VersionEntry) -> VersionView {
    VersionView {
        version: entry.version,
        path: entry.path,
        op: operation(entry.op),
        author: entry.author,
        at: entry.at,
        group: entry.group,
        reverts_version: entry.reverts_version,
        diff: entry.diff,
    }
}

pub(super) fn written(output: WriteOutput) -> Written {
    Written {
        path: output.path,
        version: output.version,
    }
}

fn versioned<T: Serialize>(status: StatusCode, version: i32, body: T) -> Response {
    let mut response = (status, Json(body)).into_response();
    response
        .headers_mut()
        .insert(ETAG, etag(VersionNumber(version)));
    response
}

async fn current_view(state: &AppState, session: &ApiSession, id: PageId) -> Option<Box<PageView>> {
    read::read_by_id(state.pool(), &session.actor, id, None)
        .await
        .ok()
        .map(|page| Box::new(page_view(id, page)))
}

async fn refine(state: &AppState, session: &ApiSession, id: PageId, error: PagesError) -> ApiError {
    match error {
        PagesError::Conflict { current_version } => ApiError::PreconditionFailed {
            version: current_version,
            current: current_view(state, session, id).await,
        },
        other => other.into(),
    }
}

#[utoipa::path(
    get,
    path = "/api/pages",
    tag = "pages",
    security(("session" = []), ("bearer" = [])),
    params(("path" = String, Query)),
    responses(
        (status = 200, body = PageView, headers(("ETag" = String))),
        (status = 400, body = Problem, content_type = "application/problem+json"),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 404, body = Problem, content_type = "application/problem+json"),
    )
)]
async fn get_page_by_path(
    State(state): State<AppState>,
    session: ApiSession,
    Query(query): Query<PathQuery>,
) -> Result<Response, ApiError> {
    let path = PagePath::parse(&query.path).map_err(|_| ApiError::InvalidInput("path"))?;
    let (id, page) = read::read_with_id(state.pool(), &session.actor, &path, None).await?;
    Ok(versioned(StatusCode::OK, page.version, page_view(id, page)))
}

#[utoipa::path(
    put,
    path = "/api/pages",
    tag = "pages",
    security(("session" = []), ("bearer" = [])),
    params(
        ("path" = String, Query),
        ("If-None-Match" = String, Header, description = "Must be `*`: the page is created only if the path is free"),
    ),
    request_body = PageCreate,
    responses(
        (status = 201, body = PageView, headers(("ETag" = String), ("Location" = String))),
        (status = 400, body = Problem, content_type = "application/problem+json"),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 403, body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "A file holds the path", body = Problem, content_type = "application/problem+json"),
        (status = 412, description = "A page holds the path; the body is that page", body = Problem),
        (status = 422, description = "The page breaks the grammar or matches a credential pattern", body = Problem, content_type = "application/problem+json"),
        (status = 428, body = Problem, content_type = "application/problem+json"),
    )
)]
async fn create_page(
    State(state): State<AppState>,
    session: ApiSession,
    Query(query): Query<PathQuery>,
    headers: HeaderMap,
    Json(request): Json<PageCreate>,
) -> Result<Response, ApiError> {
    requires_absent(&headers)?;
    let path = PagePath::parse(&query.path).map_err(|_| ApiError::InvalidInput("path"))?;
    let action = WriteAction::Create {
        content: request.content,
    };
    let created = write::write(
        state.pool(),
        state.time_zone(),
        &session.actor,
        &path,
        action,
        request.task.as_deref(),
    )
    .await;
    match created {
        Ok(_) => {
            let (id, page) = read::read_with_id(state.pool(), &session.actor, &path, None).await?;
            let location = HeaderValue::from_str(&format!("/api/pages/{}", id.0))
                .map_err(|_| ApiError::InvalidInput("path"))?;
            let version = page.version;
            let mut response = versioned(StatusCode::CREATED, version, page_view(id, page));
            response.headers_mut().insert(LOCATION, location);
            Ok(response)
        }
        Err(PagesError::PathTaken) => {
            match read::read_with_id(state.pool(), &session.actor, &path, None).await {
                Ok((id, page)) => Err(ApiError::PreconditionFailed {
                    version: VersionNumber(page.version),
                    current: Some(Box::new(page_view(id, page))),
                }),
                Err(_) => Err(PagesError::PathTaken.into()),
            }
        }
        Err(error) => Err(error.into()),
    }
}

#[utoipa::path(
    get,
    path = "/api/pages/{id}",
    tag = "pages",
    security(("session" = []), ("bearer" = [])),
    params(("id" = i64, Path), ("version" = Option<i32>, Query)),
    responses(
        (status = 200, body = PageView, headers(("ETag" = String))),
        (status = 304),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 404, body = Problem, content_type = "application/problem+json"),
        (status = 410, description = "The page was purged", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn get_page(
    State(state): State<AppState>,
    session: ApiSession,
    Path(id): Path<PageId>,
    Query(query): Query<VersionQuery>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let version = query.version.map(VersionNumber);
    let page = read::read_by_id(state.pool(), &session.actor, id, version).await?;
    let tag = etag(VersionNumber(page.version));
    if matches_none_match(&headers, &tag) {
        let mut response = StatusCode::NOT_MODIFIED.into_response();
        response.headers_mut().insert(ETAG, tag);
        return Ok(response);
    }
    Ok(versioned(StatusCode::OK, page.version, page_view(id, page)))
}

#[utoipa::path(
    put,
    path = "/api/pages/{id}",
    tag = "pages",
    security(("session" = []), ("bearer" = [])),
    params(
        ("id" = i64, Path),
        ("If-Match" = String, Header, description = "Strong ETag of the version being replaced"),
    ),
    request_body = PageOverwrite,
    responses(
        (status = 200, body = PageView, headers(("ETag" = String))),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 404, body = Problem, content_type = "application/problem+json"),
        (status = 412, description = "The page changed; the body is its current state", body = Problem),
        (status = 422, body = Problem, content_type = "application/problem+json"),
        (status = 428, body = Problem, content_type = "application/problem+json"),
    )
)]
async fn overwrite_page(
    State(state): State<AppState>,
    session: ApiSession,
    Path(id): Path<PageId>,
    headers: HeaderMap,
    Json(request): Json<PageOverwrite>,
) -> Result<Response, ApiError> {
    let revision = Revision::Overwrite {
        content: request.content,
    };
    revise(
        &state,
        &session,
        id,
        &headers,
        revision,
        request.task.as_deref(),
    )
    .await
}

#[utoipa::path(
    patch,
    path = "/api/pages/{id}",
    tag = "pages",
    security(("session" = []), ("bearer" = [])),
    params(
        ("id" = i64, Path),
        ("If-Match" = String, Header, description = "Strong ETag of the version being edited"),
    ),
    request_body = PageEdit,
    responses(
        (status = 200, body = PageView, headers(("ETag" = String))),
        (status = 400, body = Problem, content_type = "application/problem+json"),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 404, body = Problem, content_type = "application/problem+json"),
        (status = 412, description = "The page changed; the body is its current state", body = Problem),
        (status = 422, body = Problem, content_type = "application/problem+json"),
        (status = 428, body = Problem, content_type = "application/problem+json"),
    )
)]
async fn edit_page(
    State(state): State<AppState>,
    session: ApiSession,
    Path(id): Path<PageId>,
    headers: HeaderMap,
    Json(request): Json<PageEdit>,
) -> Result<Response, ApiError> {
    if request.replace.is_none() && request.append.is_none() {
        return Err(ApiError::InvalidInput("replace"));
    }
    let append = request
        .append
        .map(|body| {
            let date = body
                .date
                .as_deref()
                .map(|date| parse_iso_date(date).ok_or(ApiError::InvalidInput("date")))
                .transpose()?;
            Ok::<_, ApiError>(Append {
                text: body.text,
                source: body.source,
                date,
            })
        })
        .transpose()?;
    let revision = Revision::Edit {
        replace: request.replace.map(|body| Replacement {
            old: body.old,
            new: body.new,
        }),
        append,
    };
    revise(
        &state,
        &session,
        id,
        &headers,
        revision,
        request.task.as_deref(),
    )
    .await
}

async fn revise(
    state: &AppState,
    session: &ApiSession,
    id: PageId,
    headers: &HeaderMap,
    revision: Revision,
    task: Option<&str>,
) -> Result<Response, ApiError> {
    let precondition = if_match(headers)?;
    let current = locate::locate_live(state.pool(), &session.actor, id).await?;
    let expected = precondition.expected(current.current_version);
    let written = write::write_page(
        state.pool(),
        state.time_zone(),
        &session.actor,
        id,
        expected,
        revision,
        task,
    )
    .await;
    if let Err(error) = written {
        return Err(refine(state, session, id, error).await);
    }
    let page = read::read_by_id(state.pool(), &session.actor, id, None).await?;
    Ok(versioned(StatusCode::OK, page.version, page_view(id, page)))
}

#[utoipa::path(
    delete,
    path = "/api/pages/{id}",
    tag = "pages",
    security(("session" = []), ("bearer" = [])),
    params(
        ("id" = i64, Path),
        ("If-Match" = String, Header, description = "Strong ETag of the version being deleted"),
    ),
    responses(
        (status = 204),
        (status = 400, body = Problem, content_type = "application/problem+json"),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 404, body = Problem, content_type = "application/problem+json"),
        (status = 412, description = "The page changed; the body is its current state", body = Problem),
        (status = 428, body = Problem, content_type = "application/problem+json"),
    )
)]
async fn delete_page(
    State(state): State<AppState>,
    session: ApiSession,
    Path(id): Path<PageId>,
    headers: HeaderMap,
) -> Result<StatusCode, ApiError> {
    let precondition = if_match(&headers)?;
    let current = locate::locate_live(state.pool(), &session.actor, id).await?;
    let expected = precondition.expected(current.current_version);
    match delete::delete_page(state.pool(), &session.actor, id, expected, None).await {
        Ok(_) => Ok(StatusCode::NO_CONTENT),
        Err(error) => Err(refine(&state, &session, id, error).await),
    }
}

#[utoipa::path(
    get,
    path = "/api/pages/{id}/versions",
    tag = "pages",
    security(("session" = []), ("bearer" = [])),
    params(
        ("id" = i64, Path),
        ("limit" = Option<u32>, Query),
        ("cursor" = Option<String>, Query, description = "The `next` value of the previous page"),
    ),
    responses(
        (status = 200, body = VersionList),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 404, body = Problem, content_type = "application/problem+json"),
        (status = 410, body = Problem, content_type = "application/problem+json"),
    )
)]
async fn list_versions(
    State(state): State<AppState>,
    session: ApiSession,
    Path(id): Path<PageId>,
    Query(query): Query<VersionsQuery>,
) -> Result<Json<VersionList>, ApiError> {
    let before = query
        .cursor
        .as_deref()
        .map(|cursor| cursor.parse().map_err(|_| ApiError::InvalidInput("cursor")))
        .transpose()?;
    let page =
        history::versions_of_page(state.pool(), &session.actor, id, query.limit, before).await?;
    Ok(Json(VersionList {
        versions: page.versions.into_iter().map(version_view).collect(),
        next: page.next_before.map(|before| before.to_string()),
    }))
}

#[utoipa::path(
    post,
    path = "/api/pages/{id}/versions/{n}/restore",
    tag = "pages",
    security(("session" = []), ("bearer" = [])),
    params(
        ("id" = i64, Path),
        ("n" = i32, Path),
        ("If-Match" = String, Header, description = "Strong ETag of the current version"),
    ),
    responses(
        (status = 200, body = Restored, headers(("ETag" = String))),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 404, body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "The page already has that state, or its path is taken", body = Problem, content_type = "application/problem+json"),
        (status = 412, description = "The page changed; the body is its current state", body = Problem),
        (status = 428, body = Problem, content_type = "application/problem+json"),
    )
)]
async fn restore_version(
    State(state): State<AppState>,
    session: ApiSession,
    Path((id, n)): Path<(PageId, VersionNumber)>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let precondition = if_match(&headers)?;
    let current = locate::locate(state.pool(), &session.actor, id).await?;
    let expected = precondition.expected(current.current_version);
    let restored =
        revert::revert_page_version(state.pool(), &session.actor, id, expected, n, None).await;
    match restored {
        Ok(output) => {
            let version = output.reverted.first().map_or(0, |written| written.version);
            let reverted = output.reverted.into_iter().map(written).collect();
            Ok(versioned(StatusCode::OK, version, Restored { reverted }))
        }
        Err(error) => Err(refine(&state, &session, id, error).await),
    }
}

#[utoipa::path(
    post,
    path = "/api/pages/{id}/move",
    tag = "pages",
    security(("session" = []), ("bearer" = [])),
    params(
        ("id" = i64, Path),
        ("If-Match" = String, Header, description = "Strong ETag of the version being moved"),
    ),
    request_body = PageMove,
    responses(
        (status = 200, body = Moved, headers(("ETag" = String))),
        (status = 400, body = Problem, content_type = "application/problem+json"),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 404, body = Problem, content_type = "application/problem+json"),
        (status = 409, description = "The target path is taken", body = Problem, content_type = "application/problem+json"),
        (status = 412, description = "The page changed; the body is its current state", body = Problem),
        (status = 428, body = Problem, content_type = "application/problem+json"),
    )
)]
async fn move_page(
    State(state): State<AppState>,
    session: ApiSession,
    Path(id): Path<PageId>,
    headers: HeaderMap,
    Json(request): Json<PageMove>,
) -> Result<Response, ApiError> {
    let precondition = if_match(&headers)?;
    let to = PagePath::parse(&request.to).map_err(|_| ApiError::InvalidInput("to"))?;
    let current = locate::locate_live(state.pool(), &session.actor, id).await?;
    let expected = precondition.expected(current.current_version);
    let moved = relocate::move_page_by_id(
        state.pool(),
        &session.actor,
        id,
        expected,
        &to,
        request.task.as_deref(),
    )
    .await;
    match moved {
        Ok(output) => {
            let version = output.version;
            let view = Moved {
                id: id.0,
                path: output.path,
                version,
                rewritten: output.rewritten.into_iter().map(written).collect(),
                rewritten_out_of_scope: output.rewritten_out_of_scope,
            };
            Ok(versioned(StatusCode::OK, version, view))
        }
        Err(error) => Err(refine(&state, &session, id, error).await),
    }
}
