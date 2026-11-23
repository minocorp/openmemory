use axum::extract::{Path, Query, State};
use axum::http::header::{ETAG, LOCATION};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use memory_ops::{
    Append, FieldRead, FileSourceRead, PagePath, PageRead, Replacement, TimelineRead, VersionEntry,
    VersionOp, WriteAction, WriteOutput, parse_iso_date,
};
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};

use crate::api::error::ApiError;
use crate::api::precondition::{etag, if_match, matches_none_match, requires_absent};
use crate::auth::ApiSession;
use crate::error::ErrorBody;
use crate::pages::error::PagesError;
use crate::pages::ids::{PageId, VersionNumber};
use crate::pages::write::Revision;
use crate::pages::{delete, history, locate, read, relocate, revert, write};
use crate::state::AppState;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub(crate) struct PageView {
    id: PageId,
    path: String,
    version: i32,
    author: String,
    updated_at: String,
    fields: Vec<FieldView>,
    body: String,
    timeline: Vec<TimelineView>,
    content: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub(super) struct PreconditionFailedBody {
    error: String,
    current: Option<PageView>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
struct FieldView {
    key: String,
    values: Vec<String>,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
struct TimelineView {
    date: String,
    source: String,
    file: Option<FileSourceView>,
    text: String,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
struct FileSourceView {
    path: String,
    page: Option<u32>,
    quote: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(super) enum OperationView {
    Create,
    Edit,
    Move,
    Relink,
    Delete,
    Revert,
}

#[derive(Debug, Serialize, ToSchema)]
struct VersionView {
    version: i32,
    path: String,
    op: OperationView,
    author: String,
    at: String,
    group: i64,
    reverts_version: Option<i32>,
    diff: String,
}

#[derive(Debug, Serialize, ToSchema)]
struct VersionListView {
    versions: Vec<VersionView>,
    next_before: Option<i32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub(super) struct WrittenView {
    pub(super) path: String,
    pub(super) version: i32,
}

#[derive(Debug, Serialize, ToSchema)]
struct MovedView {
    id: PageId,
    path: String,
    version: i32,
    rewritten: Vec<WrittenView>,
    rewritten_out_of_scope: u32,
}

#[derive(Debug, Serialize, ToSchema)]
struct RestoredView {
    reverted: Vec<WrittenView>,
}

#[derive(Debug, Deserialize, ToSchema)]
struct PageCreate {
    content: String,
    task: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
struct PageOverwrite {
    content: String,
    task: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
struct ReplaceBody {
    old: String,
    new: String,
}

#[derive(Debug, Deserialize, ToSchema)]
struct AppendBody {
    text: String,
    source: Option<String>,
    date: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
struct PageEdit {
    replace: Option<ReplaceBody>,
    append: Option<AppendBody>,
    task: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
struct PageMove {
    to: String,
    task: Option<String>,
}

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
    before: Option<i32>,
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
        PreconditionFailedBody,
        FieldView,
        TimelineView,
        FileSourceView,
        OperationView,
        VersionView,
        VersionListView,
        WrittenView,
        MovedView,
        RestoredView,
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

impl PageView {
    pub(crate) fn new(id: PageId, page: PageRead) -> Self {
        Self {
            id,
            path: page.path,
            version: page.version,
            author: page.author,
            updated_at: page.updated_at,
            fields: page.fields.into_iter().map(FieldView::from).collect(),
            body: page.body,
            timeline: page.timeline.into_iter().map(TimelineView::from).collect(),
            content: page.content,
        }
    }
}

impl From<FieldRead> for FieldView {
    fn from(field: FieldRead) -> Self {
        Self {
            key: field.key,
            values: field.values,
        }
    }
}

impl From<TimelineRead> for TimelineView {
    fn from(entry: TimelineRead) -> Self {
        Self {
            date: entry.date,
            source: entry.source,
            file: entry.file.map(FileSourceView::from),
            text: entry.text,
        }
    }
}

impl From<FileSourceRead> for FileSourceView {
    fn from(source: FileSourceRead) -> Self {
        Self {
            path: source.path,
            page: source.page,
            quote: source.quote,
        }
    }
}

impl From<VersionOp> for OperationView {
    fn from(op: VersionOp) -> Self {
        match op {
            VersionOp::Create => Self::Create,
            VersionOp::Edit => Self::Edit,
            VersionOp::Move => Self::Move,
            VersionOp::Relink => Self::Relink,
            VersionOp::Delete => Self::Delete,
            VersionOp::Revert => Self::Revert,
        }
    }
}

impl From<VersionEntry> for VersionView {
    fn from(entry: VersionEntry) -> Self {
        Self {
            version: entry.version,
            path: entry.path,
            op: entry.op.into(),
            author: entry.author,
            at: entry.at,
            group: entry.group,
            reverts_version: entry.reverts_version,
            diff: entry.diff,
        }
    }
}

impl From<WriteOutput> for WrittenView {
    fn from(written: WriteOutput) -> Self {
        Self {
            path: written.path,
            version: written.version,
        }
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
        .map(|page| Box::new(PageView::new(id, page)))
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
        (status = 400, body = ErrorBody),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
    )
)]
async fn get_page_by_path(
    State(state): State<AppState>,
    session: ApiSession,
    Query(query): Query<PathQuery>,
) -> Result<Response, ApiError> {
    let path = PagePath::parse(&query.path).map_err(|_| ApiError::InvalidInput("path"))?;
    let (id, page) = read::read_with_id(state.pool(), &session.actor, &path, None).await?;
    Ok(versioned(
        StatusCode::OK,
        page.version,
        PageView::new(id, page),
    ))
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
        (status = 400, body = ErrorBody),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 409, description = "A file holds the path", body = ErrorBody),
        (status = 412, description = "A page holds the path; the body is that page", body = PreconditionFailedBody),
        (status = 422, description = "The page breaks the grammar or matches a credential pattern", body = ErrorBody),
        (status = 428, body = ErrorBody),
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
            let mut response = versioned(StatusCode::CREATED, version, PageView::new(id, page));
            response.headers_mut().insert(LOCATION, location);
            Ok(response)
        }
        Err(PagesError::PathTaken) => {
            match read::read_with_id(state.pool(), &session.actor, &path, None).await {
                Ok((id, page)) => Err(ApiError::PreconditionFailed {
                    version: VersionNumber(page.version),
                    current: Some(Box::new(PageView::new(id, page))),
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
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 410, description = "The page was purged", body = ErrorBody),
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
    Ok(versioned(
        StatusCode::OK,
        page.version,
        PageView::new(id, page),
    ))
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
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 412, description = "The page changed; the body is its current state", body = PreconditionFailedBody),
        (status = 422, body = ErrorBody),
        (status = 428, body = ErrorBody),
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
        (status = 400, body = ErrorBody),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 412, description = "The page changed; the body is its current state", body = PreconditionFailedBody),
        (status = 422, body = ErrorBody),
        (status = 428, body = ErrorBody),
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
    Ok(versioned(
        StatusCode::OK,
        page.version,
        PageView::new(id, page),
    ))
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
        (status = 400, body = ErrorBody),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 412, description = "The page changed; the body is its current state", body = PreconditionFailedBody),
        (status = 428, body = ErrorBody),
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
        ("before" = Option<i32>, Query, description = "Only versions older than this number"),
    ),
    responses(
        (status = 200, body = VersionListView),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 410, body = ErrorBody),
    )
)]
async fn list_versions(
    State(state): State<AppState>,
    session: ApiSession,
    Path(id): Path<PageId>,
    Query(query): Query<VersionsQuery>,
) -> Result<Json<VersionListView>, ApiError> {
    let page =
        history::versions_of_page(state.pool(), &session.actor, id, query.limit, query.before)
            .await?;
    Ok(Json(VersionListView {
        versions: page.versions.into_iter().map(VersionView::from).collect(),
        next_before: page.next_before,
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
        (status = 200, body = RestoredView, headers(("ETag" = String))),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 409, description = "The page already has that state, or its path is taken", body = ErrorBody),
        (status = 412, description = "The page changed; the body is its current state", body = PreconditionFailedBody),
        (status = 428, body = ErrorBody),
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
            let reverted = output.reverted.into_iter().map(WrittenView::from).collect();
            Ok(versioned(
                StatusCode::OK,
                version,
                RestoredView { reverted },
            ))
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
        (status = 200, body = MovedView, headers(("ETag" = String))),
        (status = 400, body = ErrorBody),
        (status = 401, body = ErrorBody),
        (status = 404, body = ErrorBody),
        (status = 409, description = "The target path is taken", body = ErrorBody),
        (status = 412, description = "The page changed; the body is its current state", body = PreconditionFailedBody),
        (status = 428, body = ErrorBody),
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
            let view = MovedView {
                id,
                path: output.path,
                version,
                rewritten: output
                    .rewritten
                    .into_iter()
                    .map(WrittenView::from)
                    .collect(),
                rewritten_out_of_scope: output.rewritten_out_of_scope,
            };
            Ok(versioned(StatusCode::OK, version, view))
        }
        Err(error) => Err(refine(&state, &session, id, error).await),
    }
}
