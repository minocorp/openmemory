use axum::http::StatusCode;
use sqlx::PgPool;

use crate::support::{App, TestResult};

#[sqlx::test]
async fn the_openapi_document_is_public_and_lists_every_route_group(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let reply = app.plain_get("/api/openapi.json").await?;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(
        reply.body["openapi"]
            .as_str()
            .is_some_and(|v| v.starts_with("3."))
    );
    let paths = reply.body["paths"].as_object().ok_or("no paths")?;
    for expected in [
        "/api/pages",
        "/api/pages/{id}",
        "/api/pages/{id}/versions",
        "/api/pages/{id}/versions/{n}/restore",
        "/api/pages/{id}/move",
        "/api/pages/{id}/purge",
        "/api/files",
        "/api/files/{id}",
        "/api/files/{id}/content",
        "/api/files/{id}/text",
        "/api/folder",
        "/api/files/{id}/purge",
        "/api/upload-links/{token}",
        "/api/tree",
        "/api/search",
        "/api/changes",
        "/api/changes/seen",
        "/api/changes/{group}/revert",
        "/api/users",
        "/api/users/{id}",
        "/api/system",
        "/api/system/export",
        "/api/system/import",
        "/api/purge/challenge",
        "/api/invites",
        "/api/connections",
        "/api/auth/me",
        "/api/auth/login/start",
        "/api/oauth/consents/{reference}/approve",
        "/api/openapi.json",
    ] {
        assert!(paths.contains_key(expected), "missing {expected}");
    }
    let put = &paths["/api/pages/{id}"]["put"];
    assert!(put["responses"]["412"].is_object());
    assert!(put["responses"]["428"].is_object());
    assert_eq!(
        reply.body["components"]["securitySchemes"]["session"]["name"],
        "__Host-session"
    );
    Ok(())
}
