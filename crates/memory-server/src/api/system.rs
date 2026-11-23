use axum::body::Body;
use axum::extract::State;
use axum::http::header::{CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::Response;
use axum::routing::{get, post};
use axum::{Json, Router};
use utoipa::OpenApi;

use crate::api::error::ApiError;
use crate::api::files::read_body;
use crate::auth::confirmation::{Action, Confirmed};
use crate::auth::{BinarySession, WebSession};
use crate::backup::{BackupReport, Problem};
use crate::error::ErrorBody;
use crate::state::AppState;
use crate::system::{self, DatabaseStatus, SystemStatus};
use crate::transfer::{self, ImportReport, MAX_ARCHIVE_BYTES, TransferError};

#[derive(OpenApi)]
#[openapi(
    paths(get_system, export, import),
    components(schemas(
        DatabaseStatus,
        BackupReport,
        Problem,
        SystemStatus,
        ImportReport,
        transfer::Skipped,
        transfer::SkipReason
    ))
)]
pub(super) struct Doc;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/system", get(get_system))
        .route("/api/system/export", post(export))
        .route("/api/system/import", post(import))
}

#[utoipa::path(
    get,
    path = "/api/system",
    tag = "system",
    security(("session" = [])),
    responses(
        (status = 200, body = SystemStatus),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
    )
)]
async fn get_system(
    State(state): State<AppState>,
    session: WebSession,
) -> Result<Json<SystemStatus>, ApiError> {
    session.require_admin()?;
    Ok(Json(system::status(state.pool(), state.backup()).await))
}

#[utoipa::path(
    post,
    path = "/api/system/export",
    tag = "system",
    security(("session" = [])),
    request_body = Confirmed,
    responses(
        (status = 200, description = "A zip with `pages/<path>.md` for every page and `files/<path>` for every file", content_type = "application/zip"),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
    )
)]
async fn export(
    State(state): State<AppState>,
    session: WebSession,
    Json(request): Json<Confirmed>,
) -> Result<Response, ApiError> {
    session.require_admin()?;
    request
        .confirmation
        .verify(&state, &session, Action::Export)
        .await?;
    let archive = transfer::export(state.pool(), state.files(), &session.actor).await?;
    let mut response = Response::new(Body::from_stream(archive));
    let headers = response.headers_mut();
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/zip"));
    headers.insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_static("attachment; filename=\"memory-export.zip\""),
    );
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    Ok(response)
}

#[utoipa::path(
    post,
    path = "/api/system/import",
    tag = "system",
    security(("session" = [])),
    request_body(content_type = "application/zip", description = "An archive in the export layout. This route takes the zip as the request body instead of JSON; it still needs the session cookie and a matching Origin header."),
    responses(
        (status = 200, description = "Entries that could not be imported are listed in `skipped`", body = ImportReport),
        (status = 400, body = ErrorBody),
        (status = 401, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 413, body = ErrorBody),
    )
)]
async fn import(
    State(state): State<AppState>,
    BinarySession(session): BinarySession,
    headers: HeaderMap,
    body: Body,
) -> Result<(StatusCode, Json<ImportReport>), ApiError> {
    session.require_admin()?;
    let archive = read_body(&headers, body, MAX_ARCHIVE_BYTES)
        .await
        .ok_or(TransferError::TooLarge)?;
    let report = transfer::import(
        state.pool(),
        state.time_zone(),
        state.files(),
        &session.actor,
        archive,
    )
    .await?;
    Ok((StatusCode::OK, Json(report)))
}
