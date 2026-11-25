use std::collections::HashMap;

use axum::Router;
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::http::header::{CONTENT_TYPE, ETAG};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use serde_json::{Value, json};

use super::{Handle, problem};

const PAGE_PATH: &str = "clients/acme";

pub(super) fn router() -> Router<Handle> {
    Router::new()
        .route("/api/pages", get(get_page).put(create_page))
        .route(
            "/api/pages/{id}",
            axum::routing::put(overwrite).patch(append).delete(delete),
        )
        .route("/api/pages/{id}/move", post(relocate))
        .route("/api/pages/{id}/versions", get(versions))
        .route("/api/pages/{id}/versions/{n}/restore", post(restore))
        .route("/api/folder", get(folder))
        .route("/api/search", get(search))
        .route("/api/files", get(file).post(upload))
        .route("/api/files/{id}/content", get(content))
        .route("/api/files/{id}/text", get(text))
        .route("/api/changes", get(changes))
        .route("/api/changes/seen", post(seen))
        .route("/api/items", get(items))
}

fn view(path: &str, version: i32, content: &str) -> Value {
    json!({
        "id": 7, "path": path, "version": version, "author": "mino",
        "updated_at": "2026-11-24T00:00:00Z", "fields": [], "body": content,
        "timeline": [], "content": content,
    })
}

fn with_etag(status: StatusCode, version: i32, body: &Value) -> Response {
    (
        status,
        [(ETAG, format!("\"{version}\""))],
        axum::Json(body.clone()),
    )
        .into_response()
}

async fn get_page(
    State(shared): State<Handle>,
    Query(query): Query<HashMap<String, String>>,
) -> Response {
    let state = shared.lock().unwrap();
    if query.get("path").map(String::as_str) != Some(PAGE_PATH) {
        return problem(StatusCode::NOT_FOUND, "not_found");
    }
    with_etag(
        StatusCode::OK,
        state.version,
        &view(PAGE_PATH, state.version, &state.content),
    )
}

async fn create_page(Query(query): Query<HashMap<String, String>>, body: Bytes) -> Response {
    let path = query.get("path").map_or("", String::as_str);
    if path == PAGE_PATH {
        return problem(StatusCode::PRECONDITION_FAILED, "precondition_failed");
    }
    let sent: Value = serde_json::from_slice(&body).unwrap();
    let content = sent["content"].as_str().unwrap_or_default();
    with_etag(StatusCode::CREATED, 1, &view(path, 1, content))
}

async fn overwrite(
    State(shared): State<Handle>,
    headers: axum::http::HeaderMap,
    body: Bytes,
) -> Response {
    let mut state = shared.lock().unwrap();
    let expected = format!("\"{}\"", state.version);
    if headers
        .get("if-match")
        .and_then(|value| value.to_str().ok())
        != Some(expected.as_str())
    {
        let current = view(PAGE_PATH, state.version, &state.content);
        let body =
            openmemory_api::Problem::new("precondition_failed", 412).with("current", current);
        return (
            StatusCode::PRECONDITION_FAILED,
            [(CONTENT_TYPE, "application/problem+json")],
            serde_json::to_string(&body).unwrap(),
        )
            .into_response();
    }
    let sent: Value = serde_json::from_slice(&body).unwrap();
    state.version += 1;
    state.content = sent["content"].as_str().unwrap_or_default().to_owned();
    with_etag(
        StatusCode::OK,
        state.version,
        &view(PAGE_PATH, state.version, &state.content),
    )
}

async fn append(State(shared): State<Handle>) -> Response {
    let mut state = shared.lock().unwrap();
    state.version += 1;
    state.content.push_str("- appended\n");
    with_etag(
        StatusCode::OK,
        state.version,
        &view(PAGE_PATH, state.version, &state.content),
    )
}

async fn delete() -> StatusCode {
    StatusCode::NO_CONTENT
}

async fn relocate(body: Bytes) -> axum::Json<Value> {
    let sent: Value = serde_json::from_slice(&body).unwrap();
    axum::Json(json!({
        "id": 7, "path": sent["to"], "version": 2, "rewritten": [], "rewritten_out_of_scope": 0,
    }))
}

async fn versions() -> axum::Json<Value> {
    axum::Json(json!({
        "versions": [{
            "version": 1, "path": PAGE_PATH, "op": "create", "author": "mino",
            "at": "2026-11-24T00:00:00Z", "group": 1, "reverts_version": null, "diff": "+# Acme\n",
        }],
        "next": null,
    }))
}

async fn restore(Path((_, version)): Path<(i64, i32)>) -> axum::Json<Value> {
    axum::Json(json!({ "reverted": [{ "path": PAGE_PATH, "version": version + 10 }] }))
}

async fn folder(Query(query): Query<HashMap<String, String>>) -> axum::Json<Value> {
    if query.contains_key("cursor") {
        return axum::Json(json!({
            "path": "", "entries": [{ "name": "scan.pdf", "kind": "file" }], "next": null,
        }));
    }
    axum::Json(json!({
        "path": "",
        "entries": [{ "name": "clients", "kind": "folder" }, { "name": "start", "kind": "page" }],
        "next": "c2",
    }))
}

async fn search() -> axum::Json<Value> {
    axum::Json(json!({
        "pages": [{
            "page_id": 7, "path": PAGE_PATH, "line_number": 1, "line": "# Acme",
            "author": "mino", "at": "2026-11-24T00:00:00Z",
        }],
        "files": [], "next": null, "no_hits": null,
    }))
}

fn file_view() -> Value {
    json!({
        "id": 9, "path": "scans/a.pdf", "mime": "application/pdf", "size": 7,
        "page_count": 2, "created_at": "2026-11-24T00:00:00Z",
    })
}

async fn file() -> axum::Json<Value> {
    axum::Json(file_view())
}

async fn upload(State(shared): State<Handle>, body: Bytes) -> Response {
    shared.lock().unwrap().uploaded = body.to_vec();
    (StatusCode::CREATED, axum::Json(file_view())).into_response()
}

async fn content() -> &'static str {
    "PDFDATA"
}

async fn text(Query(query): Query<HashMap<String, String>>) -> axum::Json<Value> {
    let page: u32 = query
        .get("page")
        .and_then(|page| page.parse().ok())
        .unwrap_or(1);
    axum::Json(json!({ "path": "scans/a.pdf", "page": page, "page_count": 2, "text": "hello\n" }))
}

async fn changes() -> axum::Json<Value> {
    axum::Json(json!({
        "since": 0, "last_seq": 5, "next": null,
        "groups": [{
            "id": 1, "task": null, "author": "agent", "author_display_name": "Agent",
            "at": "2026-11-24T00:00:00Z",
            "versions": [{ "page_id": 7, "path": PAGE_PATH, "version": 1, "op": "create", "seq": 5, "diff": "" }],
        }],
    }))
}

async fn seen() -> axum::Json<Value> {
    axum::Json(json!({ "last_seen_seq": 5 }))
}

async fn items(Query(query): Query<HashMap<String, String>>) -> axum::Json<Value> {
    match query.get("cursor").map(String::as_str) {
        None => axum::Json(json!({ "items": [1], "next": "b" })),
        Some(_) => axum::Json(json!({ "items": [2], "next": null })),
    }
}
