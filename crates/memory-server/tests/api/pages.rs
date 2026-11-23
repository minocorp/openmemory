use axum::http::{Method, StatusCode};
use serde_json::{Value, json};
use sqlx::PgPool;

use crate::support::{App, Browser, Reply, TestResult};

pub(crate) const PAGE: &str = "---\nstatus: active\nfee: 500000\n---\nPartner. 500,000 KRW.\n\n## Timeline\n2026-03-02 | call — signed\n";

pub(crate) async fn create(browser: &Browser, path: &str, content: &str) -> TestResult<Reply> {
    browser
        .exchange(
            Method::PUT,
            &format!("/api/pages?path={path}"),
            &[("if-none-match", "*")],
            Some(json!({"content": content})),
        )
        .await
}

pub(crate) async fn created_id(browser: &Browser, path: &str, content: &str) -> TestResult<i64> {
    let reply = create(browser, path, content).await?;
    assert_eq!(reply.status, StatusCode::CREATED, "{:?}", reply.body);
    Ok(reply.body["id"].as_i64().ok_or("no id")?)
}

fn etag(reply: &Reply) -> &str {
    reply
        .headers
        .get("etag")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
}

async fn with_match(
    browser: &Browser,
    method: Method,
    path: &str,
    version: &str,
    body: Option<Value>,
) -> TestResult<Reply> {
    browser
        .exchange(method, path, &[("if-match", version)], body)
        .await
}

#[sqlx::test]
async fn creating_a_page_returns_its_id_location_and_strong_etag(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let reply = create(&admin, "clients/acme", PAGE).await?;
    assert_eq!(reply.status, StatusCode::CREATED, "{:?}", reply.body);
    let id = reply.body["id"].as_i64().ok_or("no id")?;
    assert_eq!(etag(&reply), "\"1\"");
    assert_eq!(
        reply.headers.get("location").and_then(|v| v.to_str().ok()),
        Some(format!("/api/pages/{id}").as_str())
    );
    assert_eq!(reply.body["path"], "clients/acme");
    assert_eq!(reply.body["fields"][0]["key"], "status");
    assert_eq!(reply.body["timeline"][0]["source"], "call");

    let by_id = admin.get(&format!("/api/pages/{id}")).await?;
    let by_path = admin.get("/api/pages?path=clients/acme").await?;
    assert_eq!(by_id.status, StatusCode::OK);
    assert_eq!(by_id.body, by_path.body);
    assert_eq!(etag(&by_id), "\"1\"");
    Ok(())
}

#[sqlx::test]
async fn reading_with_a_matching_if_none_match_is_not_modified(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let id = created_id(&admin, "a", "one\n").await?;
    let same = admin
        .exchange(
            Method::GET,
            &format!("/api/pages/{id}"),
            &[("if-none-match", "\"1\"")],
            None,
        )
        .await?;
    assert_eq!(same.status, StatusCode::NOT_MODIFIED);
    let stale = admin
        .exchange(
            Method::GET,
            &format!("/api/pages/{id}"),
            &[("if-none-match", "\"0\"")],
            None,
        )
        .await?;
    assert_eq!(stale.status, StatusCode::OK);
    Ok(())
}

#[sqlx::test]
async fn creating_needs_if_none_match_and_a_free_path(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let missing = admin
        .exchange(
            Method::PUT,
            "/api/pages?path=a",
            &[],
            Some(json!({"content": "one\n"})),
        )
        .await?;
    assert_eq!(missing.status, StatusCode::PRECONDITION_REQUIRED);
    assert_eq!(missing.body["error"], "precondition_required");

    created_id(&admin, "a", "one\n").await?;
    let taken = create(&admin, "a", "two\n").await?;
    assert_eq!(taken.status, StatusCode::PRECONDITION_FAILED);
    assert_eq!(taken.body["current"]["content"], "one\n");
    assert_eq!(etag(&taken), "\"1\"");

    let bad_path = create(&admin, "a//b", "x\n").await?;
    assert_eq!(bad_path.status, StatusCode::BAD_REQUEST);
    assert_eq!(bad_path.body["field"], "path");
    Ok(())
}

#[sqlx::test]
async fn creating_over_a_file_path_conflicts_without_a_page_body(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let upload = admin
        .raw(
            Method::POST,
            "/api/files?path=contracts/acme.txt",
            &[("content-type", "text/plain")],
            b"contract".to_vec(),
        )
        .await?;
    assert_eq!(upload.status, StatusCode::CREATED);
    let reply = create(&admin, "contracts/acme.txt", "x\n").await?;
    assert_eq!(reply.status, StatusCode::CONFLICT);
    assert_eq!(reply.body["error"], "path_taken");
    Ok(())
}

#[sqlx::test]
async fn overwriting_needs_the_current_version_in_if_match(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let id = created_id(&admin, "a", PAGE).await?;
    let url = format!("/api/pages/{id}");
    let next = PAGE.replace("500,000", "450,000");

    let missing = admin
        .exchange(Method::PUT, &url, &[], Some(json!({"content": next})))
        .await?;
    assert_eq!(missing.status, StatusCode::PRECONDITION_REQUIRED);

    let wrong = with_match(
        &admin,
        Method::PUT,
        &url,
        "\"7\"",
        Some(json!({"content": next})),
    )
    .await?;
    assert_eq!(wrong.status, StatusCode::PRECONDITION_FAILED);
    assert_eq!(wrong.body["error"], "precondition_failed");
    assert_eq!(wrong.body["current"]["content"], PAGE);
    assert_eq!(etag(&wrong), "\"1\"");

    let weak = with_match(
        &admin,
        Method::PUT,
        &url,
        "W/\"1\"",
        Some(json!({"content": next})),
    )
    .await?;
    assert_eq!(
        weak.status,
        StatusCode::PRECONDITION_FAILED,
        "weak tags never match"
    );

    let right = with_match(
        &admin,
        Method::PUT,
        &url,
        "\"1\"",
        Some(json!({"content": next})),
    )
    .await?;
    assert_eq!(right.status, StatusCode::OK, "{:?}", right.body);
    assert_eq!(etag(&right), "\"2\"");
    assert_eq!(right.body["content"], next);

    let star = with_match(
        &admin,
        Method::PUT,
        &url,
        "*",
        Some(json!({"content": next.replace("450,000", "400,000")})),
    )
    .await?;
    assert_eq!(star.status, StatusCode::OK);
    assert_eq!(etag(&star), "\"3\"");
    Ok(())
}

#[sqlx::test]
async fn a_stale_second_writer_gets_the_first_writers_text(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let id = created_id(&admin, "a", PAGE).await?;
    let url = format!("/api/pages/{id}");
    let first = with_match(
        &admin,
        Method::PATCH,
        &url,
        "\"1\"",
        Some(json!({"replace": {"old": "500,000 KRW", "new": "450,000 KRW"}})),
    )
    .await?;
    assert_eq!(first.status, StatusCode::OK, "{:?}", first.body);
    let second = with_match(
        &admin,
        Method::PATCH,
        &url,
        "\"1\"",
        Some(json!({"replace": {"old": "Partner", "new": "Vendor"}})),
    )
    .await?;
    assert_eq!(second.status, StatusCode::PRECONDITION_FAILED);
    assert!(
        second.body["current"]["body"]
            .as_str()
            .is_some_and(|body| body.contains("450,000 KRW"))
    );
    assert_eq!(etag(&second), "\"2\"");
    Ok(())
}

#[sqlx::test]
async fn patching_replaces_text_and_appends_timeline_lines(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let id = created_id(&admin, "a", PAGE).await?;
    let url = format!("/api/pages/{id}");

    let both = with_match(
        &admin,
        Method::PATCH,
        &url,
        "\"1\"",
        Some(json!({
            "replace": {"old": "500,000 KRW", "new": "450,000 KRW"},
            "append": {"text": "fee lowered", "source": "call"},
            "task": "renewal talks"
        })),
    )
    .await?;
    assert_eq!(both.status, StatusCode::OK, "{:?}", both.body);
    assert_eq!(etag(&both), "\"2\"");
    assert!(
        both.body["body"]
            .as_str()
            .is_some_and(|body| body.contains("450,000 KRW"))
    );
    let timeline = both.body["timeline"].as_array().ok_or("no timeline")?;
    assert_eq!(timeline.len(), 2);
    assert_eq!(timeline[1]["text"], "fee lowered");

    let empty = with_match(&admin, Method::PATCH, &url, "\"2\"", Some(json!({}))).await?;
    assert_eq!(empty.status, StatusCode::BAD_REQUEST);

    let missing_text = with_match(
        &admin,
        Method::PATCH,
        &url,
        "\"2\"",
        Some(json!({"replace": {"old": "does not appear", "new": "x"}})),
    )
    .await?;
    assert_eq!(missing_text.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(missing_text.body["reason"], "not_found");

    let bad_line = with_match(
        &admin,
        Method::PATCH,
        &url,
        "\"2\"",
        Some(json!({"append": {"text": "x", "source": ""}})),
    )
    .await?;
    assert_eq!(bad_line.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(bad_line.body["error"], "grammar");
    assert!(bad_line.body["line"].is_u64());
    Ok(())
}

#[sqlx::test]
async fn appended_lines_take_an_event_date_that_is_not_in_the_future(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let id = created_id(&admin, "a", PAGE).await?;
    let url = format!("/api/pages/{id}");

    let past = with_match(
        &admin,
        Method::PATCH,
        &url,
        "\"1\"",
        Some(
            json!({"append": {"text": "contract signed", "source": "call", "date": "2026-01-15"}}),
        ),
    )
    .await?;
    assert_eq!(past.status, StatusCode::OK, "{:?}", past.body);
    assert_eq!(past.body["timeline"][0]["date"], "2026-01-15");
    assert_eq!(past.body["timeline"][0]["text"], "contract signed");

    let future = with_match(
        &admin,
        Method::PATCH,
        &url,
        "\"2\"",
        Some(json!({"append": {"text": "later", "date": "2999-01-01"}})),
    )
    .await?;
    assert_eq!(future.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(future.body["error"], "future_date");
    assert!(future.body["today"].is_string());

    let invalid = with_match(
        &admin,
        Method::PATCH,
        &url,
        "\"2\"",
        Some(json!({"append": {"text": "x", "date": "15/01/2026"}})),
    )
    .await?;
    assert_eq!(invalid.status, StatusCode::BAD_REQUEST);
    assert_eq!(invalid.body["field"], "date");
    Ok(())
}

#[sqlx::test]
async fn a_quote_that_is_not_in_the_cited_file_page_is_rejected(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let id = created_id(&admin, "a", PAGE).await?;
    let url = format!("/api/pages/{id}");
    let upload = admin
        .raw(
            Method::POST,
            "/api/files?path=notes/call.txt",
            &[("content-type", "text/plain")],
            b"The monthly fee is 450,000 won.".to_vec(),
        )
        .await?;
    assert_eq!(upload.status, StatusCode::CREATED);

    let wrong = with_match(
        &admin,
        Method::PATCH,
        &url,
        "\"1\"",
        Some(json!({"append": {"text": "x", "source": "notes/call.txt#p1 \"fee is 400,000\""}})),
    )
    .await?;
    assert_eq!(wrong.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(wrong.body["error"], "quote_unverified");
    assert_eq!(wrong.body["reason"], "not_in_text");

    let right = with_match(
        &admin,
        Method::PATCH,
        &url,
        "\"1\"",
        Some(json!({"append": {"text": "x", "source": "notes/call.txt#p1 \"Fee is 450,000\""}})),
    )
    .await?;
    assert_eq!(right.status, StatusCode::OK, "{:?}", right.body);
    Ok(())
}

#[sqlx::test]
async fn overwriting_cannot_rewrite_timeline_history(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let id = created_id(&admin, "a", PAGE).await?;
    let rewritten = PAGE.replace("signed", "never signed");
    let reply = with_match(
        &admin,
        Method::PUT,
        &format!("/api/pages/{id}"),
        "\"1\"",
        Some(json!({"content": rewritten})),
    )
    .await?;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(reply.body["error"], "timeline_changed");
    Ok(())
}

#[sqlx::test]
async fn secrets_are_rejected_with_the_pattern_name(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let reply = create(&admin, "a", "card 4111 1111 1111 1111\n").await?;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(reply.body["error"], "credential");
    assert_eq!(reply.body["pattern"], "card number");
    Ok(())
}

#[sqlx::test]
async fn deleting_and_restoring_keep_the_id_and_history(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let id = created_id(&admin, "a", PAGE).await?;
    let url = format!("/api/pages/{id}");

    let missing = admin.exchange(Method::DELETE, &url, &[], None).await?;
    assert_eq!(missing.status, StatusCode::PRECONDITION_REQUIRED);
    let stale = with_match(&admin, Method::DELETE, &url, "\"9\"", None).await?;
    assert_eq!(stale.status, StatusCode::PRECONDITION_FAILED);
    let deleted = with_match(&admin, Method::DELETE, &url, "\"1\"", None).await?;
    assert_eq!(deleted.status, StatusCode::NO_CONTENT);

    assert_eq!(admin.get(&url).await?.status, StatusCode::NOT_FOUND);
    let versions = admin.get(&format!("{url}/versions")).await?;
    assert_eq!(versions.status, StatusCode::OK);
    assert_eq!(versions.body["versions"][0]["op"], "delete");

    let restored = with_match(
        &admin,
        Method::POST,
        &format!("{url}/versions/1/restore"),
        "\"2\"",
        None,
    )
    .await?;
    assert_eq!(restored.status, StatusCode::OK, "{:?}", restored.body);
    assert_eq!(etag(&restored), "\"3\"");
    let back = admin.get(&url).await?;
    assert_eq!(back.status, StatusCode::OK);
    assert_eq!(back.body["content"], PAGE);
    assert_eq!(back.body["id"], id);
    Ok(())
}

#[sqlx::test]
async fn restoring_needs_a_fresh_if_match_and_a_different_state(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let id = created_id(&admin, "a", PAGE).await?;
    let url = format!("/api/pages/{id}");
    let edit = with_match(
        &admin,
        Method::PATCH,
        &url,
        "\"1\"",
        Some(json!({"replace": {"old": "Partner", "new": "Vendor"}})),
    )
    .await?;
    assert_eq!(edit.status, StatusCode::OK);

    let missing = admin
        .exchange(
            Method::POST,
            &format!("{url}/versions/1/restore"),
            &[],
            None,
        )
        .await?;
    assert_eq!(missing.status, StatusCode::PRECONDITION_REQUIRED);
    let stale = with_match(
        &admin,
        Method::POST,
        &format!("{url}/versions/1/restore"),
        "\"1\"",
        None,
    )
    .await?;
    assert_eq!(stale.status, StatusCode::PRECONDITION_FAILED);
    assert_eq!(stale.body["current"]["version"], 2);

    let unknown = with_match(
        &admin,
        Method::POST,
        &format!("{url}/versions/9/restore"),
        "\"2\"",
        None,
    )
    .await?;
    assert_eq!(unknown.status, StatusCode::NOT_FOUND);
    assert_eq!(unknown.body["error"], "version_not_found");

    let same = with_match(
        &admin,
        Method::POST,
        &format!("{url}/versions/2/restore"),
        "\"2\"",
        None,
    )
    .await?;
    assert_eq!(same.status, StatusCode::CONFLICT);
    assert_eq!(same.body["error"], "already_current");

    let ok = with_match(
        &admin,
        Method::POST,
        &format!("{url}/versions/1/restore"),
        "\"2\"",
        None,
    )
    .await?;
    assert_eq!(ok.status, StatusCode::OK);
    assert_eq!(admin.get(&url).await?.body["content"], PAGE);
    Ok(())
}

#[sqlx::test]
async fn versions_come_newest_first_with_diffs_and_a_cursor(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let id = created_id(&admin, "a", PAGE).await?;
    let url = format!("/api/pages/{id}");
    for (version, old, new) in [
        ("\"1\"", "Partner", "Vendor"),
        ("\"2\"", "Vendor", "Supplier"),
    ] {
        let reply = with_match(
            &admin,
            Method::PATCH,
            &url,
            version,
            Some(json!({"replace": {"old": old, "new": new}})),
        )
        .await?;
        assert_eq!(reply.status, StatusCode::OK);
    }

    let first = admin.get(&format!("{url}/versions?limit=2")).await?;
    let listed = first.body["versions"].as_array().ok_or("no versions")?;
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0]["version"], 3);
    assert_eq!(listed[0]["op"], "edit");
    let diff = listed[0]["diff"].as_str().ok_or("no diff")?;
    assert!(diff.contains("-Vendor. 500,000 KRW."), "{diff}");
    assert!(diff.contains("+Supplier. 500,000 KRW."), "{diff}");
    assert_eq!(first.body["next_before"], 2);

    let rest = admin
        .get(&format!("{url}/versions?limit=2&before=2"))
        .await?;
    let remaining = rest.body["versions"].as_array().ok_or("no versions")?;
    assert_eq!(remaining.len(), 1);
    assert_eq!(remaining[0]["op"], "create");
    assert!(rest.body["next_before"].is_null());
    Ok(())
}

#[sqlx::test]
async fn moving_rewrites_links_in_other_pages(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let target = created_id(&admin, "clients/acme", PAGE).await?;
    let linker = created_id(&admin, "notes/hub", "See [[clients/acme]].\n").await?;
    created_id(&admin, "clients/taken", "x\n").await?;
    let url = format!("/api/pages/{target}/move");

    let missing = admin
        .exchange(Method::POST, &url, &[], Some(json!({"to": "archive/acme"})))
        .await?;
    assert_eq!(missing.status, StatusCode::PRECONDITION_REQUIRED);
    let stale = with_match(
        &admin,
        Method::POST,
        &url,
        "\"4\"",
        Some(json!({"to": "archive/acme"})),
    )
    .await?;
    assert_eq!(stale.status, StatusCode::PRECONDITION_FAILED);
    let taken = with_match(
        &admin,
        Method::POST,
        &url,
        "\"1\"",
        Some(json!({"to": "clients/taken"})),
    )
    .await?;
    assert_eq!(taken.status, StatusCode::CONFLICT);
    let invalid = with_match(
        &admin,
        Method::POST,
        &url,
        "\"1\"",
        Some(json!({"to": "../x"})),
    )
    .await?;
    assert_eq!(invalid.status, StatusCode::BAD_REQUEST);

    let moved = with_match(
        &admin,
        Method::POST,
        &url,
        "\"1\"",
        Some(json!({"to": "archive/acme"})),
    )
    .await?;
    assert_eq!(moved.status, StatusCode::OK, "{:?}", moved.body);
    assert_eq!(moved.body["id"], target);
    assert_eq!(moved.body["path"], "archive/acme");
    assert_eq!(moved.body["rewritten"][0]["path"], "notes/hub");
    assert_eq!(etag(&moved), "\"2\"");

    let hub = admin.get(&format!("/api/pages/{linker}")).await?;
    assert_eq!(hub.body["content"], "See [[archive/acme]].\n");
    assert_eq!(
        admin.get(&format!("/api/pages/{target}")).await?.body["path"],
        "archive/acme"
    );
    Ok(())
}

#[sqlx::test]
async fn page_routes_need_a_session_origin_and_json(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let id = created_id(&admin, "a", "one\n").await?;
    let anonymous = app.browser();
    assert_eq!(
        anonymous.get(&format!("/api/pages/{id}")).await?.status,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        anonymous.get("/api/pages?path=a").await?.status,
        StatusCode::UNAUTHORIZED
    );

    let without_origin = axum::http::Request::builder()
        .method(Method::PATCH)
        .uri(format!("/api/pages/{id}"))
        .header("cookie", admin.clone_cookie())
        .header("content-type", "application/json")
        .header("if-match", "\"1\"")
        .body(axum::body::Body::from(
            json!({"append": {"text": "x"}}).to_string(),
        ))?;
    let forbidden =
        crate::support::read(tower::ServiceExt::oneshot(app.router.clone(), without_origin).await?)
            .await?;
    assert_eq!(forbidden.status, StatusCode::FORBIDDEN);

    let plain = admin
        .raw(
            Method::PATCH,
            &format!("/api/pages/{id}"),
            &[("content-type", "text/plain"), ("if-match", "\"1\"")],
            b"x".to_vec(),
        )
        .await?;
    assert_eq!(plain.status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    Ok(())
}

#[sqlx::test]
async fn unknown_ids_and_paths_are_not_found(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    assert_eq!(
        admin.get("/api/pages/999").await?.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        admin.get("/api/pages?path=nope").await?.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        admin.get("/api/pages?path=a//b").await?.status,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        admin.get("/api/does-not-exist").await?.status,
        StatusCode::NOT_FOUND
    );
    Ok(())
}
