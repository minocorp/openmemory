use axum::body::Body;
use axum::http::{Request, StatusCode};
use rmcp::model::Tool;
use serde_json::json;
use sqlx::PgPool;
use tower::ServiceExt;

use super::harness::{Server, full_agent, grant_for};
use crate::support::{App, TestResult};

const INITIALIZE: &str = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"test","version":"1"}}}"#;

fn mcp_request(token: Option<&str>, origin: Option<&str>) -> TestResult<Request<Body>> {
    let mut builder = Request::post("/mcp")
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .header("host", "localhost:8080");
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    if let Some(origin) = origin {
        builder = builder.header("origin", origin);
    }
    Ok(builder.body(Body::from(INITIALIZE))?)
}

fn tool<'a>(tools: &'a [Tool], name: &str) -> TestResult<&'a Tool> {
    Ok(tools
        .iter()
        .find(|tool| tool.name == name)
        .ok_or("tool missing")?)
}

#[sqlx::test]
async fn exactly_the_eight_operations_are_exposed_with_annotations(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let server = Server::start(&app).await?;
    let (mut admin, _) = app.admin().await?;
    let agent = full_agent(&app, &server, &mut admin, "claude-code").await?;

    let tools = agent.client.list_all_tools().await?;
    let mut names: Vec<_> = tools.iter().map(|tool| tool.name.to_string()).collect();
    names.sort();
    assert_eq!(
        names,
        [
            "delete", "history", "move", "read", "revert", "search", "upload", "write"
        ]
    );
    let expectations = [
        ("read", true, None),
        ("search", true, None),
        ("write", false, Some(true)),
        ("revert", false, Some(true)),
        ("delete", false, Some(true)),
        ("move", false, Some(false)),
        ("upload", false, Some(false)),
        ("history", true, None),
    ];
    for (name, read_only, destructive) in expectations {
        let annotations = tool(&tools, name)?
            .annotations
            .clone()
            .ok_or("no annotations")?;
        assert_eq!(annotations.read_only_hint, Some(read_only), "{name}");
        assert_eq!(annotations.destructive_hint, destructive, "{name}");
        assert_eq!(annotations.open_world_hint, Some(false), "{name}");
        let description = tool(&tools, name)?
            .description
            .clone()
            .ok_or("no description")?;
        assert!(
            description.contains("read the page `start` first"),
            "{name}"
        );
        assert!(description.contains("dates are ISO"), "{name}");
    }
    let write = serde_json::to_value(tool(&tools, "write")?.input_schema.as_ref())?;
    assert_eq!(write["required"], json!(["path"]));
    assert!(write["properties"]["base_version"]["description"].is_string());
    let search = serde_json::to_value(tool(&tools, "search")?.input_schema.as_ref())?;
    assert!(search["properties"].get("where").is_some());
    Ok(())
}

#[sqlx::test]
async fn server_instructions_are_the_start_page_or_an_interview_prompt(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let server = Server::start(&app).await?;
    let (mut admin, _) = app.admin().await?;
    let agent = full_agent(&app, &server, &mut admin, "claude-code").await?;

    let before = agent
        .client
        .peer_info()
        .and_then(|info| info.instructions.clone());
    assert!(
        before.is_some_and(|text| text.contains("Interview the owner")),
        "default instructions"
    );
    agent
        .ok(
            "write",
            json!({"path": "start", "content": "Dates are ISO.\nFolders: customers\n"}),
        )
        .await?;

    let second = server
        .agent(grant_for(&app, &mut admin, "second", &["customers"], false).await?)
        .await?;
    let after = second
        .client
        .peer_info()
        .and_then(|info| info.instructions.clone());
    assert_eq!(
        after.as_deref(),
        Some("Dates are ISO.\nFolders: customers\n")
    );
    Ok(())
}

#[sqlx::test]
async fn requests_without_a_valid_token_get_401_with_the_resource_metadata_challenge(
    pool: PgPool,
) -> TestResult {
    let app = App::new(pool);

    let missing = app.router.clone().oneshot(mcp_request(None, None)?).await?;
    assert_eq!(missing.status(), StatusCode::UNAUTHORIZED);
    let challenge = missing.headers()["www-authenticate"].to_str()?;
    assert_eq!(
        challenge,
        r#"Bearer resource_metadata="http://localhost:8080/.well-known/oauth-protected-resource/mcp""#
    );

    let wrong = app
        .router
        .clone()
        .oneshot(mcp_request(Some("not-a-token"), None)?)
        .await?;
    assert_eq!(wrong.status(), StatusCode::UNAUTHORIZED);
    assert!(
        wrong.headers()["www-authenticate"]
            .to_str()?
            .contains(r#"error="invalid_token""#)
    );
    Ok(())
}

#[sqlx::test]
async fn foreign_origins_are_rejected_and_the_public_origin_is_accepted(
    pool: PgPool,
) -> TestResult {
    let app = App::new(pool);
    let (mut admin, _) = app.admin().await?;
    let grant = grant_for(&app, &mut admin, "claude-code", &["*"], true).await?;

    let foreign = app
        .router
        .clone()
        .oneshot(mcp_request(
            Some(&grant.access),
            Some("https://evil.example"),
        )?)
        .await?;
    assert_eq!(foreign.status(), StatusCode::FORBIDDEN);
    let own = app
        .router
        .clone()
        .oneshot(mcp_request(
            Some(&grant.access),
            Some("http://localhost:8080"),
        )?)
        .await?;
    assert_eq!(own.status(), StatusCode::OK);
    let none = app
        .router
        .clone()
        .oneshot(mcp_request(Some(&grant.access), None)?)
        .await?;
    assert_eq!(none.status(), StatusCode::OK);
    let other_host = Request::post("/mcp")
        .header("content-type", "application/json")
        .header("accept", "application/json, text/event-stream")
        .header("host", "attacker.example")
        .header("authorization", format!("Bearer {}", grant.access))
        .body(Body::from(INITIALIZE))?;
    let rebound = app.router.clone().oneshot(other_host).await?;
    assert_eq!(rebound.status(), StatusCode::FORBIDDEN);
    Ok(())
}

#[sqlx::test]
async fn revoking_a_connection_stops_its_token_at_once(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let server = Server::start(&app).await?;
    let (mut admin, _) = app.admin().await?;
    let agent = full_agent(&app, &server, &mut admin, "claude-code").await?;
    agent.ok("read", json!({"path": "/"})).await?;

    let connections = admin.get("/api/connections").await?;
    let id = connections.body[0]["id"].as_i64().ok_or("no id")?;
    let removed = admin.delete(&format!("/api/connections/{id}")).await?;
    assert_eq!(removed.status, StatusCode::NO_CONTENT);

    assert!(agent.call("read", json!({"path": "/"})).await.is_err());
    let reply = app
        .router
        .clone()
        .oneshot(mcp_request(Some(&agent.grant.access), None)?)
        .await?;
    assert_eq!(reply.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}
