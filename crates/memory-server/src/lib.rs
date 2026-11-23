mod access;
mod api;
mod auth;
mod backup;
mod config;
mod db;
mod error;
mod files;
mod health;
mod jobs;
mod mcp;
mod oauth;
mod pages;
mod purge;
mod rate_limit;
mod search;
mod security_headers;
mod state;
mod storage;
mod system;
mod telemetry;
#[cfg(test)]
mod test_support;
mod time_zone;
mod transfer;
mod tree;
mod web;

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use axum::Router;
use axum::http::StatusCode;
use axum::middleware;
use tokio::net::TcpListener;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

pub use config::Config;
pub use state::AppState;
pub use telemetry::init_tracing;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub fn router(state: AppState) -> Router {
    let headers = security_headers::SecurityHeaders::for_origin(state.public_url());
    Router::new()
        .merge(health::router())
        .merge(auth::router())
        .merge(oauth::router())
        .merge(mcp::router(&state))
        .merge(access::connections::router())
        .merge(api::router())
        .fallback(web::serve)
        .layer(middleware::from_fn_with_state(
            state.rate_limits().clone(),
            rate_limit::enforce,
        ))
        .layer(TimeoutLayer::with_status_code(
            StatusCode::REQUEST_TIMEOUT,
            REQUEST_TIMEOUT,
        ))
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn_with_state(
            headers,
            security_headers::apply,
        ))
        .with_state(state)
}

pub fn openapi_json() -> serde_json::Result<String> {
    serde_json::to_string_pretty(&api::openapi())
}

pub async fn run(config: Config) -> anyhow::Result<()> {
    let pool = db::connect(config.database_url())
        .await
        .context("connect to database")?;
    db::migrate(&pool).await.context("run migrations")?;
    config
        .time_zone()
        .ensure_known(&pool)
        .await
        .context("check time zone")?;
    let store = storage::build(config.storage())
        .await
        .context("build blob storage")?;
    let listener = TcpListener::bind(config.listen())
        .await
        .context("bind listen address")?;
    tracing::info!(listen = %config.listen(), "server started");
    let state = AppState::new(pool.clone(), Arc::clone(&store), config.max_upload_bytes())
        .with_public_url(config.public_url().clone())
        .context("configure public url")?
        .with_time_zone(config.time_zone().clone())
        .with_trusted_proxies(config.trusted_proxies())
        .with_client_ip_header(config.client_ip_header().cloned())
        .with_backup(backup::Backup::from_config(config.backup()));
    if let Some(link) = state.bootstrap_link().await.context("arm bootstrap")? {
        tracing::info!(%link, "no users yet: open this link to create the first admin");
    }
    let jobs = jobs::Jobs::start(
        pool,
        store,
        state.rate_limits().clone(),
        state.backup().clone(),
    );
    let served = axum::serve(
        listener,
        router(state).into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .context("serve");
    jobs.shutdown().await;
    served
}

async fn shutdown_signal() {
    let mut terminate =
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(signal) => signal,
            Err(error) => {
                tracing::error!(%error, "install SIGTERM handler");
                return;
            }
        };
    tokio::select! {
        result = tokio::signal::ctrl_c() => {
            if let Err(error) = result {
                tracing::error!(%error, "wait for ctrl-c");
            }
        }
        _ = terminate.recv() => {}
    }
}
