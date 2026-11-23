use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use crate::backup::BackupReport;
use crate::state::AppState;
use crate::system::{self, DatabaseStatus};

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
enum Status {
    Ok,
    Degraded,
    Unavailable,
}

#[derive(Debug, Serialize)]
struct Health {
    status: Status,
    database: DatabaseStatus,
    backup: Option<BackupReport>,
}

pub(crate) fn router() -> Router<AppState> {
    Router::new().route("/healthz", get(healthz))
}

async fn healthz(State(state): State<AppState>) -> (StatusCode, Json<Health>) {
    let system = system::status(state.pool(), state.backup()).await;
    let (code, status) = match system.database {
        DatabaseStatus::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, Status::Unavailable),
        DatabaseStatus::Ok
            if system
                .backup
                .as_ref()
                .is_some_and(BackupReport::is_degraded) =>
        {
            (StatusCode::OK, Status::Degraded)
        }
        DatabaseStatus::Ok => (StatusCode::OK, Status::Ok),
    };
    let health = Health {
        status,
        database: system.database,
        backup: system.backup,
    };
    (code, Json(health))
}

#[cfg(test)]
mod tests {
    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use object_store::memory::InMemory;
    use sqlx::PgPool;
    use std::sync::Arc;
    use tower::ServiceExt;

    use crate::backup::Backup;

    use super::*;

    async fn get(pool: PgPool, backup: Backup) -> serde_json::Value {
        let state = AppState::new(pool, Arc::new(InMemory::new()), 1024).with_backup(backup);
        let response = crate::router(state)
            .oneshot(Request::get("/healthz").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&body).unwrap()
    }

    #[sqlx::test]
    async fn unmonitored_backups_leave_the_status_ok(pool: PgPool) {
        let body = get(pool, Backup::default()).await;
        assert_eq!(body["status"], "ok");
        assert!(body["backup"].is_null());
    }

    #[sqlx::test]
    async fn a_missing_backup_status_degrades_health_and_lists_the_problem(pool: PgPool) {
        let directory = tempfile::tempdir().unwrap();
        let backup = Backup::watching(&directory.path().join("status.json"));
        let body = get(pool, backup).await;
        assert_eq!(body["status"], "degraded");
        assert_eq!(body["database"], "ok");
        assert_eq!(body["backup"]["problems"][0], "status_missing");
    }
}
