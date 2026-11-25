mod headers;

use axum::body::{Body, to_bytes};
use axum::extract::{Path, Query, State};
use axum::http::header::{CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, IF_RANGE, LOCATION, RANGE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use axum::{Json, Router};
use bytes::Bytes;
use openmemory_api::{FileText, FileView};
use serde::Deserialize;
use utoipa::OpenApi;

use crate::api::error::ApiError;
use crate::api::files::headers::with_content_headers;
use crate::api::precondition::matches_none_match;
use crate::api::range::{self, RangeRequest};
use crate::auth::{ApiSession, BinaryApiSession};
use crate::files::{FileContent, FileId, FileRecord, FileRef, FilesError};
use crate::state::AppState;
use openmemory_api::Problem;

#[derive(Debug, Deserialize)]
struct PathQuery {
    path: String,
}

#[derive(Debug, Deserialize)]
struct TextQuery {
    page: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ContentQuery {
    download: Option<bool>,
}

#[derive(OpenApi)]
#[openapi(
    paths(
        upload_file,
        get_file_by_path,
        get_file,
        get_file_content,
        get_file_text,
        delete_file
    ),
    components(schemas(FileView, FileText))
)]
pub(super) struct Doc;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/files", get(get_file_by_path).post(upload_file))
        .route("/api/files/{id}", get(get_file).delete(delete_file))
        .route("/api/files/{id}/content", get(get_file_content))
        .route("/api/files/{id}/text", get(get_file_text))
}

pub(super) fn file_view(record: FileRecord) -> FileView {
    FileView {
        id: record.id.0,
        path: record.path.to_string(),
        mime: record.mime,
        size: record.size,
        page_count: record.page_count,
        created_at: record.created_at,
    }
}

pub(super) async fn read_body(headers: &HeaderMap, body: Body, limit: u64) -> Option<Bytes> {
    let declared = headers
        .get(CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok());
    if declared.is_some_and(|length| length > limit) {
        return None;
    }
    let cap = usize::try_from(limit.saturating_add(1)).unwrap_or(usize::MAX);
    to_bytes(body, cap)
        .await
        .ok()
        .filter(|bytes| u64::try_from(bytes.len()).is_ok_and(|length| length <= limit))
}

pub(super) fn content_type(headers: &HeaderMap) -> Result<&str, ApiError> {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .ok_or(ApiError::InvalidInput("content-type"))
}

pub(super) fn created(record: FileRecord) -> Result<Response, ApiError> {
    let location = HeaderValue::from_str(&format!("/api/files/{}", record.id.0))
        .map_err(|_| ApiError::InvalidInput("path"))?;
    let mut response = (StatusCode::CREATED, Json(file_view(record))).into_response();
    response.headers_mut().insert(LOCATION, location);
    Ok(response)
}

#[utoipa::path(
    post,
    path = "/api/files",
    tag = "files",
    security(("session" = []), ("bearer" = [])),
    params(
        ("path" = String, Query),
        ("Content-Type" = String, Header, description = "The MIME type of the file"),
    ),
    request_body(content_type = "application/octet-stream", description = "The raw file bytes. This route takes the file as the request body instead of JSON; it still needs the session cookie and a matching Origin header."),
    responses(
        (status = 201, body = FileView, headers(("Location" = String))),
        (status = 400, body = Problem, content_type = "application/problem+json"),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 403, body = Problem, content_type = "application/problem+json"),
        (status = 409, body = Problem, content_type = "application/problem+json"),
        (status = 413, body = Problem, content_type = "application/problem+json"),
        (status = 415, body = Problem, content_type = "application/problem+json"),
        (status = 422, body = Problem, content_type = "application/problem+json"),
    )
)]
async fn upload_file(
    State(state): State<AppState>,
    BinaryApiSession(session): BinaryApiSession,
    Query(query): Query<PathQuery>,
    headers: HeaderMap,
    body: Body,
) -> Result<Response, ApiError> {
    let mime = content_type(&headers)?;
    let limit = state.files().max_upload_bytes();
    let bytes = read_body(&headers, body, limit)
        .await
        .ok_or(FilesError::TooLarge { limit })?;
    let record = state
        .files()
        .upload(&session.actor, &query.path, FileContent::Bytes(bytes), mime)
        .await?;
    created(record)
}

#[utoipa::path(
    get,
    path = "/api/files",
    tag = "files",
    security(("session" = []), ("bearer" = [])),
    params(("path" = String, Query)),
    responses(
        (status = 200, body = FileView),
        (status = 400, body = Problem, content_type = "application/problem+json"),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 404, body = Problem, content_type = "application/problem+json"),
    )
)]
async fn get_file_by_path(
    State(state): State<AppState>,
    session: ApiSession,
    Query(query): Query<PathQuery>,
) -> Result<Json<FileView>, ApiError> {
    let path =
        openmemory_ops::PagePath::parse(&query.path).map_err(|_| ApiError::InvalidInput("path"))?;
    let record = state
        .files()
        .metadata(&session.actor, &FileRef::Path(path))
        .await?;
    Ok(Json(file_view(record)))
}

#[utoipa::path(
    get,
    path = "/api/files/{id}",
    tag = "files",
    security(("session" = []), ("bearer" = [])),
    params(("id" = i64, Path)),
    responses(
        (status = 200, body = FileView),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 404, body = Problem, content_type = "application/problem+json"),
        (status = 410, body = Problem, content_type = "application/problem+json"),
    )
)]
async fn get_file(
    State(state): State<AppState>,
    session: ApiSession,
    Path(id): Path<FileId>,
) -> Result<Json<FileView>, ApiError> {
    let record = state
        .files()
        .metadata(&session.actor, &FileRef::Id(id))
        .await?;
    Ok(Json(file_view(record)))
}

#[utoipa::path(
    get,
    path = "/api/files/{id}/content",
    tag = "files",
    security(("session" = []), ("bearer" = [])),
    params(
        ("id" = i64, Path),
        ("download" = Option<bool>, Query, description = "Force a download instead of inline display"),
        ("Range" = Option<String>, Header, description = "A single byte range"),
    ),
    responses(
        (status = 200, description = "The file bytes", content_type = "application/octet-stream", headers(("ETag" = String), ("Accept-Ranges" = String), ("Content-Disposition" = String))),
        (status = 206, description = "The requested byte range", content_type = "application/octet-stream", headers(("Content-Range" = String))),
        (status = 304),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 404, body = Problem, content_type = "application/problem+json"),
        (status = 410, body = Problem, content_type = "application/problem+json"),
        (status = 416, description = "The range starts beyond the end of the file"),
    )
)]
async fn get_file_content(
    State(state): State<AppState>,
    session: ApiSession,
    Path(id): Path<FileId>,
    Query(query): Query<ContentQuery>,
    headers: HeaderMap,
) -> Result<Response, ApiError> {
    let file = FileRef::Id(id);
    let record = state.files().metadata(&session.actor, &file).await?;
    let tag = file_etag(&record);
    if matches_none_match(&headers, &tag) {
        return Ok(with_content_headers(
            StatusCode::NOT_MODIFIED.into_response(),
            &record,
            &tag,
            &headers,
            query.download,
        ));
    }
    let size = u64::try_from(record.size).unwrap_or(0);
    let requested = requested_range(&headers, &tag, size);
    let (status, length, content_range, download) = match requested {
        RangeRequest::Whole => (
            StatusCode::OK,
            size,
            None,
            state.files().download(&session.actor, &file).await?,
        ),
        RangeRequest::Partial(span) => (
            StatusCode::PARTIAL_CONTENT,
            span.end - span.start,
            Some(format!("bytes {}-{}/{size}", span.start, span.end - 1)),
            state
                .files()
                .download_range(&session.actor, &file, span)
                .await?,
        ),
        RangeRequest::Unsatisfiable => {
            let mut response = StatusCode::RANGE_NOT_SATISFIABLE.into_response();
            if let Ok(value) = HeaderValue::from_str(&format!("bytes */{size}")) {
                response.headers_mut().insert(CONTENT_RANGE, value);
            }
            return Ok(with_content_headers(
                response,
                &record,
                &tag,
                &headers,
                query.download,
            ));
        }
    };
    let served = download.record;
    let mut response = Response::new(Body::from_stream(download.stream));
    *response.status_mut() = status;
    response
        .headers_mut()
        .insert(CONTENT_LENGTH, HeaderValue::from(length));
    if let Some(value) = content_range.and_then(|text| HeaderValue::from_str(&text).ok()) {
        response.headers_mut().insert(CONTENT_RANGE, value);
    }
    Ok(with_content_headers(
        response,
        &served,
        &file_etag(&served),
        &headers,
        query.download,
    ))
}

fn file_etag(record: &FileRecord) -> HeaderValue {
    HeaderValue::from_str(&format!("\"{}\"", record.sha256.to_hex()))
        .unwrap_or_else(|_| HeaderValue::from_static("\"\""))
}

fn requested_range(headers: &HeaderMap, tag: &HeaderValue, size: u64) -> RangeRequest {
    let if_range_matches = headers
        .get(IF_RANGE)
        .is_none_or(|value| value.as_bytes() == tag.as_bytes());
    if !if_range_matches {
        return RangeRequest::Whole;
    }
    range::parse(
        headers.get(RANGE).and_then(|value| value.to_str().ok()),
        size,
    )
}

#[utoipa::path(
    get,
    path = "/api/files/{id}/text",
    tag = "files",
    security(("session" = []), ("bearer" = [])),
    params(
        ("id" = i64, Path),
        ("page" = Option<u32>, Query, description = "Text page number, from 1"),
    ),
    responses(
        (status = 200, body = FileText),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 404, description = "No such file or text page", body = Problem, content_type = "application/problem+json"),
        (status = 422, description = "The file has no extracted text pages", body = Problem, content_type = "application/problem+json"),
    )
)]
async fn get_file_text(
    State(state): State<AppState>,
    session: ApiSession,
    Path(id): Path<FileId>,
    Query(query): Query<TextQuery>,
) -> Result<Json<FileText>, ApiError> {
    let file = FileRef::Id(id);
    let record = state.files().metadata(&session.actor, &file).await?;
    let page_count = u32::try_from(record.page_count).unwrap_or(u32::MAX);
    if page_count == 0 {
        return Err(ApiError::NoTextPages);
    }
    let page = query.page.unwrap_or(1);
    let text = state.files().page_text(&session.actor, &file, page).await?;
    Ok(Json(FileText {
        path: record.path.to_string(),
        page,
        page_count,
        text,
    }))
}

#[utoipa::path(
    delete,
    path = "/api/files/{id}",
    tag = "files",
    security(("session" = []), ("bearer" = [])),
    params(("id" = i64, Path)),
    responses(
        (status = 204),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 403, body = Problem, content_type = "application/problem+json"),
        (status = 404, body = Problem, content_type = "application/problem+json"),
    )
)]
async fn delete_file(
    State(state): State<AppState>,
    session: ApiSession,
    Path(id): Path<FileId>,
) -> Result<StatusCode, ApiError> {
    state
        .files()
        .delete(&session.actor, &FileRef::Id(id))
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
