mod api;
mod oauth;

use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::http::header::CONTENT_TYPE;
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use memory_api::Problem;
use tokio::net::TcpListener;

#[derive(Debug, Clone)]
pub(crate) struct Seen {
    pub(crate) method: String,
    pub(crate) target: String,
    pub(crate) if_match: Option<String>,
    pub(crate) content_type: Option<String>,
    pub(crate) body: Vec<u8>,
}

pub(crate) struct Shared {
    pub(crate) host: String,
    pub(crate) seen: Vec<Seen>,
    pub(crate) access: HashSet<String>,
    pub(crate) refresh: HashSet<String>,
    pub(crate) refresh_calls: u32,
    pub(crate) issued: u32,
    pub(crate) challenge: Option<String>,
    pub(crate) revoked: Vec<String>,
    pub(crate) device_pending: u32,
    pub(crate) version: i32,
    pub(crate) content: String,
    pub(crate) uploaded: Vec<u8>,
}

pub(crate) type Handle = Arc<Mutex<Shared>>;

pub(crate) struct Mock {
    pub(crate) host: String,
    pub(crate) shared: Handle,
}

impl Mock {
    pub(crate) async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let host = format!("http://{}", listener.local_addr().unwrap());
        let shared = Arc::new(Mutex::new(Shared {
            host: host.clone(),
            seen: Vec::new(),
            access: HashSet::new(),
            refresh: HashSet::new(),
            refresh_calls: 0,
            issued: 0,
            challenge: None,
            revoked: Vec::new(),
            device_pending: 0,
            version: 1,
            content: "# Acme\n".to_owned(),
            uploaded: Vec::new(),
        }));
        let app = Router::new()
            .merge(api::router())
            .merge(oauth::router())
            .layer(middleware::from_fn_with_state(Arc::clone(&shared), gate))
            .with_state(Arc::clone(&shared));
        tokio::spawn(async move { axum::serve(listener, app).await });
        Self { host, shared }
    }

    pub(crate) fn seen(&self) -> Vec<Seen> {
        self.shared.lock().unwrap().seen.clone()
    }

    pub(crate) fn allow(&self, access: &str) {
        self.shared.lock().unwrap().access.insert(access.to_owned());
    }

    pub(crate) fn allow_refresh(&self, refresh: &str) {
        self.shared
            .lock()
            .unwrap()
            .refresh
            .insert(refresh.to_owned());
    }
}

pub(crate) fn problem(status: StatusCode, code: &str) -> Response {
    let body = Problem::new(code, status.as_u16());
    (
        status,
        [(CONTENT_TYPE, "application/problem+json")],
        serde_json::to_string(&body).unwrap(),
    )
        .into_response()
}

async fn gate(State(shared): State<Handle>, request: Request, next: Next) -> Response {
    let (parts, body) = request.into_parts();
    let bytes = to_bytes(body, 64 * 1024 * 1024).await.unwrap();
    let header = |name: &str| {
        parts
            .headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
    };
    let bearer =
        header("authorization").and_then(|value| value.strip_prefix("Bearer ").map(str::to_owned));
    let target = parts
        .uri
        .path_and_query()
        .map_or_else(|| parts.uri.path().to_owned(), ToString::to_string);
    let authorized = {
        let mut state = shared.lock().unwrap();
        state.seen.push(Seen {
            method: parts.method.to_string(),
            target,
            if_match: header("if-match"),
            content_type: header("content-type"),
            body: bytes.to_vec(),
        });
        bearer.is_some_and(|token| state.access.contains(&token))
    };
    if parts.uri.path().starts_with("/api/") && !authorized {
        return problem(StatusCode::UNAUTHORIZED, "unauthenticated");
    }
    next.run(Request::from_parts(parts, Body::from(bytes)))
        .await
}
