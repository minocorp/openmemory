use serde_json::json;
use sqlx::PgPool;

use super::harness::{ACME, Server, full_agent, grant_for};
use crate::support::{App, TestResult};

#[sqlx::test]
async fn scoped_connections_cannot_see_or_change_other_folders(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let server = Server::start(&app).await?;
    let (mut admin, _) = app.admin().await?;
    let full = full_agent(&app, &server, &mut admin, "full").await?;
    full.ok("write", json!({"path": "tax/2026", "content": ACME}))
        .await?;
    full.ok("write", json!({"path": "clients/acme", "content": ACME}))
        .await?;
    full.ok(
        "write",
        json!({"path": "start", "content": "Folders: tax, clients\n"}),
    )
    .await?;
    let scoped = server
        .agent(grant_for(&app, &mut admin, "tax-only", &["tax"], true).await?)
        .await?;

    let hidden = scoped
        .rejected("read", json!({"path": "clients/acme"}))
        .await?;
    assert!(hidden.contains("Nothing at `clients/acme`"), "{hidden}");
    scoped.ok("read", json!({"path": "tax/2026"})).await?;
    scoped.ok("read", json!({"path": "start"})).await?;

    let top = scoped.ok("read", json!({"path": "/"})).await?;
    assert_eq!(
        top["result"]["entries"],
        json!([{"name": "tax", "kind": "folder"}])
    );
    let found = scoped
        .ok("search", json!({"text": "Design partner"}))
        .await?;
    let paths: Vec<_> = found["result"]["hits"]
        .as_array()
        .ok_or("no hits")?
        .iter()
        .map(|hit| hit["path"].clone())
        .collect();
    assert_eq!(paths, [json!("tax/2026")]);
    let history = scoped.ok("history", json!({})).await?;
    assert!(
        history["result"]["versions"]
            .as_array()
            .ok_or("no versions")?
            .iter()
            .all(|version| version["path"] == "tax/2026")
    );

    let outside = scoped
        .rejected("write", json!({"path": "clients/new", "content": "x"}))
        .await?;
    assert!(
        outside.contains("outside this connection's folder scope"),
        "{outside}"
    );
    let moving_out = scoped
        .rejected("move", json!({"from": "tax/2026", "to": "clients/2026"}))
        .await?;
    assert!(
        moving_out.contains("outside this connection's folder scope"),
        "{moving_out}"
    );
    let upload = scoped
        .rejected("upload", json!({"path": "clients/a.txt", "text": "x"}))
        .await?;
    assert!(
        upload.contains("Not allowed to write this path"),
        "{upload}"
    );
    Ok(())
}

#[sqlx::test]
async fn read_only_connections_read_but_never_write(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let server = Server::start(&app).await?;
    let (mut admin, _) = app.admin().await?;
    let full = full_agent(&app, &server, &mut admin, "full").await?;
    full.ok("write", json!({"path": "customers/acme", "content": ACME}))
        .await?;
    let viewer = server
        .agent(grant_for(&app, &mut admin, "viewer", &["*"], false).await?)
        .await?;

    viewer.ok("read", json!({"path": "customers/acme"})).await?;
    viewer.ok("search", json!({"text": "Design"})).await?;
    let attempts = [
        (
            "write",
            json!({"path": "customers/acme", "append_text": "x"}),
        ),
        ("write", json!({"path": "customers/new", "content": "x"})),
        (
            "move",
            json!({"from": "customers/acme", "to": "customers/b"}),
        ),
        ("delete", json!({"path": "customers/acme"})),
        ("revert", json!({"path": "customers/acme", "version": 1})),
    ];
    for (tool, arguments) in attempts {
        let message = viewer.rejected(tool, arguments).await?;
        assert!(message.contains("read-only"), "{tool}: {message}");
    }
    let link = viewer
        .rejected("upload", json!({"path": "contracts/a.pdf"}))
        .await?;
    assert!(link.contains("Not allowed to write this path"), "{link}");
    Ok(())
}

#[sqlx::test]
async fn a_stale_overwrite_conflicts_and_says_how_to_recover(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let server = Server::start(&app).await?;
    let (mut admin, _) = app.admin().await?;
    let agent = full_agent(&app, &server, &mut admin, "claude-code").await?;
    agent
        .ok("write", json!({"path": "customers/acme", "content": ACME}))
        .await?;
    agent
        .ok(
            "write",
            json!({"path": "customers/acme", "append_text": "renewed"}),
        )
        .await?;

    let stale = agent
        .rejected(
            "write",
            json!({"path": "customers/acme", "content": "x", "base_version": 1}),
        )
        .await?;
    assert!(stale.contains("now at version 2"), "{stale}");
    assert!(stale.contains("`read` it again"), "{stale}");
    let page = agent.ok("read", json!({"path": "customers/acme"})).await?;
    let content = page["result"]["content"].as_str().ok_or("no content")?;
    let current = agent
        .ok(
            "write",
            json!({
                "path": "customers/acme",
                "content": content.replace("Design partner", "Partner"),
                "base_version": 2,
            }),
        )
        .await?;
    assert_eq!(current["version"], 3);

    let ambiguous = agent
        .rejected(
            "write",
            json!({"path": "customers/acme", "replace_old": "e", "replace_new": "E"}),
        )
        .await?;
    assert!(ambiguous.contains("more than once"), "{ambiguous}");
    let broken = agent
        .rejected(
            "write",
            json!({"path": "customers/bad", "content": "---\nstatus: [a\n---\nx"}),
        )
        .await?;
    assert!(broken.contains("line"), "{broken}");
    let no_action = agent
        .rejected("write", json!({"path": "customers/acme"}))
        .await?;
    assert!(no_action.contains("Invalid write"), "{no_action}");
    Ok(())
}

#[sqlx::test]
async fn zero_hit_searches_explain_how_the_query_was_read(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let server = Server::start(&app).await?;
    let (mut admin, _) = app.admin().await?;
    let full = full_agent(&app, &server, &mut admin, "full").await?;
    full.ok("write", json!({"path": "tax/2026", "content": ACME}))
        .await?;
    full.ok("write", json!({"path": "clients/acme", "content": ACME}))
        .await?;
    let scoped = server
        .agent(grant_for(&app, &mut admin, "tax-only", &["tax"], true).await?)
        .await?;

    let none = scoped
        .ok(
            "search",
            json!({"text": "nothing like this", "where": ["renewal<today"]}),
        )
        .await?;
    let explanation = &none["result"]["no_hits"];
    assert!(
        explanation["query"]
            .as_str()
            .is_some_and(|query| query.contains("literal substring, case-sensitive"))
    );
    assert_eq!(
        explanation["fields_in_scope"],
        json!(["fee", "renewal", "status"])
    );
    assert_eq!(explanation["folders_out_of_scope"], 1);

    let bad = scoped
        .rejected("search", json!({"where": ["renewal"]}))
        .await?;
    assert!(bad.contains("Invalid `where`"), "{bad}");
    Ok(())
}
