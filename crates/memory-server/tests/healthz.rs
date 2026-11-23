use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use std::sync::Arc;

use memory_server::{AppState, router};
use object_store::memory::InMemory;
use sqlx::PgPool;
use tower::ServiceExt;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

async fn get_healthz(pool: PgPool) -> TestResult<(StatusCode, serde_json::Value)> {
    let response = router(AppState::new(pool, Arc::new(InMemory::new()), 1024))
        .oneshot(Request::get("/healthz").body(Body::empty())?)
        .await?;
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await?;
    Ok((status, serde_json::from_slice(&body)?))
}

#[sqlx::test]
async fn healthz_reports_database_ok(pool: PgPool) -> TestResult {
    let (status, body) = get_healthz(pool).await?;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["database"], "ok");
    Ok(())
}

#[sqlx::test]
async fn healthz_reports_unavailable_when_database_is_closed(pool: PgPool) -> TestResult {
    pool.close().await;
    let (status, body) = get_healthz(pool).await?;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["database"], "unavailable");
    Ok(())
}
