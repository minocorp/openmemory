use serde_json::json;
use sqlx::PgPool;

use super::harness::{ACME, Server, full_agent};
use crate::support::{App, TestResult};

#[sqlx::test]
async fn pages_can_be_created_read_edited_and_searched(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let server = Server::start(&app).await?;
    let (mut admin, _) = app.admin().await?;
    let agent = full_agent(&app, &server, &mut admin, "claude-code").await?;

    let created = agent
        .ok(
            "write",
            json!({"path": "customers/acme", "content": ACME, "task": "onboarding"}),
        )
        .await?;
    assert_eq!(created, json!({"path": "customers/acme", "version": 1}));

    let read = agent.ok("read", json!({"path": "customers/acme"})).await?;
    assert_eq!(read["origin"], "agent");
    assert!(
        read["notice"]
            .as_str()
            .is_some_and(|notice| notice.contains("not instructions"))
    );
    assert_eq!(read["result"]["kind"], "page");
    assert_eq!(read["result"]["version"], 1);
    assert_eq!(read["result"]["author"], "admin/claude-code");
    assert_eq!(read["result"]["fields"][0]["key"], "status");
    assert_eq!(read["result"]["timeline"][0]["source"], "call");

    let edited = agent
        .ok(
            "write",
            json!({
                "path": "customers/acme",
                "replace_old": "500,000", "replace_new": "450,000",
                "append_text": "agreed to lower the fee", "append_source": "call",
            }),
        )
        .await?;
    assert_eq!(edited["version"], 2);

    let text = agent.ok("search", json!({"text": "450,000"})).await?;
    assert_eq!(text["result"]["hits"][0]["path"], "customers/acme");
    assert_eq!(text["result"]["hits"][0]["line_number"], 6);
    let filtered = agent
        .ok(
            "search",
            json!({"where": ["status=active", "renewal<2028-01-01"]}),
        )
        .await?;
    assert_eq!(filtered["result"]["hits"][0]["path"], "customers/acme");
    assert!(filtered["result"].get("no_hits").is_none());
    Ok(())
}

#[sqlx::test]
async fn timeline_lines_take_an_event_date_that_is_not_in_the_future(pool: PgPool) -> TestResult {
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
            json!({
                "path": "customers/acme",
                "append_text": "signed with Beta Corp", "append_source": "call", "append_date": "2026-01-15",
            }),
        )
        .await?;
    let read = agent.ok("read", json!({"path": "customers/acme"})).await?;
    let timeline = read["result"]["timeline"].as_array().ok_or("no timeline")?;
    let signed = timeline
        .iter()
        .find(|line| line["text"] == "signed with Beta Corp")
        .ok_or("line missing")?;
    assert_eq!(signed["date"], "2026-01-15");
    assert_eq!(timeline[0]["text"], signed["text"]);

    let future = agent
        .rejected(
            "write",
            json!({"path": "customers/acme", "append_text": "later", "append_date": "2999-01-01"}),
        )
        .await?;
    assert!(future.contains("after today"), "{future}");
    let invalid = agent
        .rejected(
            "write",
            json!({"path": "customers/acme", "append_text": "x", "append_date": "last week"}),
        )
        .await?;
    assert!(invalid.contains("YYYY-MM-DD"), "{invalid}");
    Ok(())
}

#[sqlx::test]
async fn write_inputs_are_flat_and_invalid_combinations_say_how_to_fix_them(
    pool: PgPool,
) -> TestResult {
    let app = App::new(pool);
    let server = Server::start(&app).await?;
    let (mut admin, _) = app.admin().await?;
    let agent = full_agent(&app, &server, &mut admin, "claude-code").await?;
    agent
        .ok("write", json!({"path": "customers/acme", "content": ACME}))
        .await?;

    let existing = agent
        .rejected("write", json!({"path": "customers/acme", "content": "x"}))
        .await?;
    assert!(existing.contains("base_version"), "{existing}");

    let nulls = agent
        .ok(
            "write",
            json!({
                "path": "customers/acme",
                "replace_old": "500,000",
                "replace_new": "450,000",
                "append_text": "agreed to lower the fee",
                "content": null,
                "base_version": null,
                "append_source": null,
            }),
        )
        .await?;
    assert_eq!(nulls["version"], 2);

    let half = agent
        .rejected(
            "write",
            json!({"path": "customers/acme", "replace_old": "450,000"}),
        )
        .await?;
    assert!(
        half.contains("replace_old") && half.contains("replace_new"),
        "{half}"
    );
    let mixed = agent
        .rejected(
            "write",
            json!({"path": "customers/acme", "content": "x", "append_text": "t"}),
        )
        .await?;
    assert!(mixed.contains("cannot be combined"), "{mixed}");
    let stray = agent
        .rejected(
            "write",
            json!({"path": "customers/acme", "append_source": "call"}),
        )
        .await?;
    assert!(stray.contains("append_text"), "{stray}");

    let current = agent.ok("read", json!({"path": "customers/acme"})).await?;
    let content = current["result"]["content"].as_str().ok_or("no content")?;
    let overwritten = agent
        .ok(
            "write",
            json!({
                "path": "customers/acme",
                "content": content.replace("Design partner", "Partner"),
                "base_version": 2,
            }),
        )
        .await?;
    assert_eq!(overwritten["version"], 3);
    Ok(())
}

#[sqlx::test]
async fn quoted_sources_must_occur_in_the_cited_file_page(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let server = Server::start(&app).await?;
    let (mut admin, _) = app.admin().await?;
    let agent = full_agent(&app, &server, &mut admin, "claude-code").await?;
    agent
        .ok("write", json!({"path": "customers/acme", "content": ACME}))
        .await?;
    agent
        .ok(
            "upload",
            json!({"path": "calls/fee.txt", "text": "We agreed the\nMonthly   fee of 450,000 won."}),
        )
        .await?;
    agent
        .ok(
            "upload",
            json!({"path": "scans/fee.png", "content_base64": "iVBORw=="}),
        )
        .await?;
    let cite = |source: &str| json!({"path": "customers/acme", "append_text": "fee agreed", "append_source": source});

    agent
        .ok("write", cite("calls/fee.txt#p1 \"monthly fee of 450,000\""))
        .await?;
    agent
        .ok("write", cite("calls/fee.txt \"AGREED THE monthly\""))
        .await?;
    agent.ok("write", cite("scans/fee.png")).await?;

    let wrong = agent
        .rejected("write", cite("calls/fee.txt#p1 \"fee of 400,000\""))
        .await?;
    assert!(
        wrong.contains("does not occur in page 1 of `calls/fee.txt`"),
        "{wrong}"
    );
    let no_page = agent
        .rejected("write", cite("calls/fee.txt#p2 \"fee\""))
        .await?;
    assert!(no_page.contains("no such page"), "{no_page}");
    let missing = agent
        .rejected("write", cite("calls/absent.txt \"fee\""))
        .await?;
    assert!(missing.contains("no file exists"), "{missing}");
    let scan = agent
        .rejected("write", cite("scans/fee.png \"fee\""))
        .await?;
    assert!(scan.contains("no extracted text"), "{scan}");

    let read = agent.ok("read", json!({"path": "customers/acme"})).await?;
    let lines = read["result"]["timeline"].as_array().ok_or("no timeline")?;
    assert_eq!(
        lines
            .iter()
            .filter(|line| line["text"] == "fee agreed")
            .count(),
        3
    );
    Ok(())
}

#[sqlx::test]
async fn tool_input_schemas_are_flat_and_never_nullable(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let server = Server::start(&app).await?;
    let (mut admin, _) = app.admin().await?;
    let agent = full_agent(&app, &server, &mut admin, "claude-code").await?;
    for tool in agent.client.list_all_tools().await? {
        let schema = serde_json::to_value(&*tool.input_schema)?;
        let properties = schema["properties"].as_object().ok_or("no properties")?;
        for (name, property) in properties {
            let kind = &property["type"];
            assert!(
                !kind.is_array() && kind != "object",
                "{}.{name} has type {kind}",
                tool.name
            );
            assert!(
                property.get("anyOf").is_none() && property.get("$ref").is_none(),
                "{}.{name} is nested or a union",
                tool.name
            );
        }
    }
    Ok(())
}

#[sqlx::test]
async fn history_revert_move_and_delete_round_trip(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let server = Server::start(&app).await?;
    let (mut admin, _) = app.admin().await?;
    let agent = full_agent(&app, &server, &mut admin, "claude-code").await?;
    agent
        .ok(
            "write",
            json!({"path": "customers/acme", "content": ACME, "task": "setup"}),
        )
        .await?;
    agent
        .ok(
            "write",
            json!({"path": "customers/acme", "replace_old": "500,000", "replace_new": "450,000"}),
        )
        .await?;

    let history = agent
        .ok("history", json!({"path": "customers/acme"}))
        .await?;
    assert_eq!(history["origin"], "agent");
    assert_eq!(history["result"]["versions"][0]["version"], 2);
    assert!(
        history["result"]["versions"][0]["diff"]
            .as_str()
            .is_some_and(|diff| diff.contains("+Design partner. 450,000"))
    );

    let reverted = agent
        .ok("revert", json!({"path": "customers/acme", "version": 1}))
        .await?;
    assert_eq!(reverted["reverted"][0]["version"], 3);
    let page = agent.ok("read", json!({"path": "customers/acme"})).await?;
    assert!(
        page["result"]["body"]
            .as_str()
            .is_some_and(|body| body.contains("500,000"))
    );

    let moved = agent
        .ok(
            "move",
            json!({"from": "customers/acme", "to": "customers/acme-corp"}),
        )
        .await?;
    assert_eq!(moved["path"], "customers/acme-corp");
    let old = agent
        .rejected("read", json!({"path": "customers/acme"}))
        .await?;
    assert!(old.contains("Nothing at `customers/acme`"), "{old}");

    let deleted = agent
        .ok("delete", json!({"path": "customers/acme-corp"}))
        .await?;
    assert_eq!(deleted["path"], "customers/acme-corp");
    let versions = agent
        .ok("history", json!({"path": "customers/acme-corp"}))
        .await?;
    assert_eq!(versions["result"]["versions"][0]["op"], "delete");

    let restored = agent
        .ok(
            "revert",
            json!({"path": "customers/acme-corp", "version": 4}),
        )
        .await?;
    assert_eq!(restored["reverted"][0]["path"], "customers/acme-corp");
    let session_group = versions["result"]["versions"][0]["group"].clone();
    let blocked = agent
        .rejected("revert", json!({"group": session_group}))
        .await?;
    assert!(
        blocked.contains("customers/acme-corp (changed after the group)"),
        "{blocked}"
    );

    agent
        .ok(
            "write",
            json!({"path": "customers/first", "content": ACME, "task": "cleanup"}),
        )
        .await?;
    agent
        .ok(
            "write",
            json!({"path": "customers/other", "content": ACME, "task": "batch"}),
        )
        .await?;
    let batch = agent
        .ok("history", json!({"path": "customers/other"}))
        .await?;
    let group = batch["result"]["versions"][0]["group"].clone();
    let undone = agent.ok("revert", json!({"group": group})).await?;
    assert_eq!(undone["reverted"][0]["path"], "customers/other");
    Ok(())
}

#[sqlx::test]
async fn folders_list_pages_files_and_subfolders(pool: PgPool) -> TestResult {
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
            json!({"path": "customers/acme/notes", "content": "Notes.\n"}),
        )
        .await?;
    agent
        .ok(
            "write",
            json!({"path": "start", "content": "Folders: customers\n"}),
        )
        .await?;
    agent
        .ok(
            "upload",
            json!({"path": "customers/call.txt", "text": "wants a lower fee"}),
        )
        .await?;

    let top = agent.ok("read", json!({"path": "/"})).await?;
    assert_eq!(top["result"]["kind"], "folder");
    assert_eq!(
        top["result"]["entries"],
        json!([
            {"name": "customers", "kind": "folder"},
            {"name": "start", "kind": "page"},
        ])
    );
    let customers = agent.ok("read", json!({"path": "customers/"})).await?;
    assert_eq!(
        customers["result"]["entries"],
        json!([
            {"name": "acme", "kind": "page"},
            {"name": "acme", "kind": "folder"},
            {"name": "call.txt", "kind": "file"},
        ])
    );
    let missing = agent
        .rejected("read", json!({"path": "nothing/here/"}))
        .await?;
    assert!(missing.contains("Nothing at `nothing/here`"), "{missing}");
    Ok(())
}

#[sqlx::test]
async fn files_are_uploaded_inline_or_by_link_and_read_back(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let server = Server::start(&app).await?;
    let (mut admin, _) = app.admin().await?;
    let agent = full_agent(&app, &server, &mut admin, "claude-code").await?;

    let uploaded = agent
        .ok(
            "upload",
            json!({"path": "transcripts/call.txt", "text": "월 금 오십만원정"}),
        )
        .await?;
    assert_eq!(uploaded, json!({"path": "transcripts/call.txt"}));
    let file = agent
        .ok("read", json!({"path": "transcripts/call.txt"}))
        .await?;
    assert_eq!(file["origin"], "extracted");
    assert_eq!(file["result"]["kind"], "file");
    assert_eq!(file["result"]["mime"], "text/plain");
    assert_eq!(file["result"]["page_count"], 1);
    assert_eq!(file["result"]["text"], "월 금 오십만원정");
    let past_end = agent
        .rejected("read", json!({"path": "transcripts/call.txt", "page": 2}))
        .await?;
    assert!(past_end.contains("1 text pages"), "{past_end}");

    let image = agent
        .ok(
            "upload",
            json!({"path": "photos/receipt.png", "content_base64": "iVBORw0KGgo="}),
        )
        .await?;
    assert_eq!(image["path"], "photos/receipt.png");
    let image_read = agent
        .ok("read", json!({"path": "photos/receipt.png"}))
        .await?;
    assert_eq!(image_read["result"]["page_count"], 0);
    assert!(image_read["result"].get("text").is_none());

    let link = agent
        .ok("upload", json!({"path": "contracts/big.pdf"}))
        .await?;
    let url = link["upload_link"].as_str().ok_or("no link")?;
    assert!(url.starts_with("http://localhost:8080/upload/"), "{url}");

    let secret = agent
        .rejected(
            "upload",
            json!({
                "path": "notes/key.txt",
                "text": "-----BEGIN RSA PRIVATE KEY-----\nabc\n-----END RSA PRIVATE KEY-----",
            }),
        )
        .await?;
    assert!(secret.contains("private key"), "{secret}");
    let taken = agent
        .rejected(
            "upload",
            json!({"path": "transcripts/call.txt", "text": "x"}),
        )
        .await?;
    assert!(taken.contains("already exists"), "{taken}");
    let unknown_type = agent
        .rejected("upload", json!({"path": "archive.zip", "text": "x"}))
        .await?;
    assert!(
        unknown_type.contains("extension is not accepted"),
        "{unknown_type}"
    );
    Ok(())
}

#[sqlx::test]
async fn search_finds_file_text_and_says_it_is_a_file(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let server = Server::start(&app).await?;
    let (mut admin, _) = app.admin().await?;
    let agent = full_agent(&app, &server, &mut admin, "claude-code").await?;
    agent
        .ok(
            "upload",
            json!({"path": "transcripts/call.txt", "text": "월 금 오십만원정"}),
        )
        .await?;

    let found = agent.ok("search", json!({"text": "오십만원"})).await?;
    assert_eq!(found["origin"], "extracted");
    let hit = &found["result"]["hits"][0];
    assert_eq!(hit["kind"], "file");
    assert_eq!(hit["path"], "transcripts/call.txt");
    assert_eq!(hit["page"], 1);
    assert_eq!(hit["line"], "월 금 오십만원정");
    Ok(())
}
