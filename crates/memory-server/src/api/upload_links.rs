use axum::body::Body;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::response::Response;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use time::OffsetDateTime;
use utoipa::{OpenApi, ToSchema};

use crate::api::error::ApiError;
use crate::api::files::{content_type, created, read_body};
use crate::error::ErrorBody;
use crate::files::FilesError;
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
struct UploadLinkView {
    path: String,
    #[serde(with = "time::serde::rfc3339")]
    expires_at: OffsetDateTime,
}

#[derive(OpenApi)]
#[openapi(
    paths(get_upload_link, redeem_upload_link),
    components(schemas(UploadLinkView))
)]
pub(super) struct Doc;

pub(super) fn router() -> Router<AppState> {
    Router::new().route(
        "/api/upload-links/{token}",
        get(get_upload_link).post(redeem_upload_link),
    )
}

#[utoipa::path(
    get,
    path = "/api/upload-links/{token}",
    tag = "upload-links",
    params(("token" = String, Path)),
    responses(
        (status = 200, body = UploadLinkView),
        (status = 404, description = "The link is unknown, expired or used", body = ErrorBody),
    )
)]
async fn get_upload_link(
    State(state): State<AppState>,
    Path(token): Path<String>,
) -> Result<Json<UploadLinkView>, ApiError> {
    let link = state.files().open_upload_link(&token).await?;
    Ok(Json(UploadLinkView {
        path: link.path.to_string(),
        expires_at: link.expires_at,
    }))
}

#[utoipa::path(
    post,
    path = "/api/upload-links/{token}",
    tag = "upload-links",
    params(
        ("token" = String, Path),
        ("Content-Type" = String, Header, description = "The MIME type of the file"),
    ),
    request_body(content_type = "application/octet-stream", description = "The raw file bytes. The token authorizes the request, so no session is needed."),
    responses(
        (status = 201, body = crate::api::files::FileView),
        (status = 400, body = ErrorBody),
        (status = 404, description = "The link is unknown, expired or used", body = ErrorBody),
        (status = 409, body = ErrorBody),
        (status = 413, body = ErrorBody),
        (status = 503, description = "Too many uploads are in flight", body = ErrorBody),
        (status = 415, body = ErrorBody),
        (status = 422, body = ErrorBody),
    )
)]
async fn redeem_upload_link(
    State(state): State<AppState>,
    Path(token): Path<String>,
    headers: HeaderMap,
    body: Body,
) -> Result<Response, ApiError> {
    state.files().open_upload_link(&token).await?;
    let mime = content_type(&headers)?;
    let _permit = state.files().reserve_upload_body().ok_or(ApiError::Busy)?;
    let limit = state.files().max_upload_bytes();
    let bytes = read_body(&headers, body, limit)
        .await
        .ok_or(FilesError::TooLarge { limit })?;
    let record = state
        .files()
        .redeem_upload_link(&token, bytes, mime)
        .await?;
    created(record)
}

#[cfg(test)]
mod tests;
