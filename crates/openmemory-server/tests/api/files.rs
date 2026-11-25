use axum::http::{Method, StatusCode};
use sqlx::PgPool;

use crate::support::{App, Browser, Raw, TestResult};

const TEXT: &[u8] = b"0123456789abcdefghij";

async fn upload(browser: &Browser, path: &str, mime: &str, bytes: &[u8]) -> TestResult<Raw> {
    browser
        .raw(
            Method::POST,
            &format!("/api/files?path={path}"),
            &[("content-type", mime)],
            bytes.to_vec(),
        )
        .await
}

async fn uploaded_id(browser: &Browser, path: &str) -> TestResult<i64> {
    let reply = upload(browser, path, "text/plain", TEXT).await?;
    assert_eq!(reply.status, StatusCode::CREATED);
    let body: serde_json::Value = serde_json::from_slice(&reply.body)?;
    Ok(body["id"].as_i64().ok_or("no id")?)
}

#[sqlx::test]
async fn uploading_takes_the_raw_body_and_returns_the_record(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let reply = upload(&admin, "notes/a.txt", "text/plain; charset=utf-8", TEXT).await?;
    assert_eq!(reply.status, StatusCode::CREATED);
    let body: serde_json::Value = serde_json::from_slice(&reply.body)?;
    let id = body["id"].as_i64().ok_or("no id")?;
    assert_eq!(body["path"], "notes/a.txt");
    assert_eq!(body["mime"], "text/plain");
    assert_eq!(body["size"], 20);
    assert_eq!(reply.header("location"), format!("/api/files/{id}"));

    let by_id = admin.get(&format!("/api/files/{id}")).await?;
    let by_path = admin.get("/api/files?path=notes/a.txt").await?;
    assert_eq!(by_id.status, StatusCode::OK);
    assert_eq!(by_id.body, by_path.body);
    Ok(())
}

#[sqlx::test]
async fn uploads_still_need_a_session_and_the_origin(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let anonymous = app.browser();
    let reply = upload(&anonymous, "a.txt", "text/plain", TEXT).await?;
    assert_eq!(reply.status, StatusCode::UNAUTHORIZED);

    let foreign = axum::http::Request::builder()
        .method(Method::POST)
        .uri("/api/files?path=a.txt")
        .header("cookie", admin.clone_cookie())
        .header("origin", "https://evil.example")
        .header("content-type", "text/plain")
        .body(axum::body::Body::from(TEXT))?;
    let blocked =
        crate::support::read(tower::ServiceExt::oneshot(app.router.clone(), foreign).await?)
            .await?;
    assert_eq!(blocked.status, StatusCode::FORBIDDEN);
    Ok(())
}

#[sqlx::test]
async fn uploads_are_validated(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    assert_eq!(
        upload(&admin, "a.zip", "application/zip", b"PK")
            .await?
            .status,
        StatusCode::UNSUPPORTED_MEDIA_TYPE
    );
    assert_eq!(
        upload(&admin, "a.txt", "text/plain", &[b'x'; 2000])
            .await?
            .status,
        StatusCode::PAYLOAD_TOO_LARGE
    );
    assert_eq!(
        upload(&admin, "a.txt", "text/plain", b"").await?.status,
        StatusCode::BAD_REQUEST
    );
    assert_eq!(
        upload(&admin, "a//b.txt", "text/plain", TEXT).await?.status,
        StatusCode::BAD_REQUEST
    );
    let secret = upload(
        &admin,
        "key.txt",
        "text/plain",
        b"-----BEGIN PRIVATE KEY-----\nabc\n-----END PRIVATE KEY-----\n",
    )
    .await?;
    assert_eq!(secret.status, StatusCode::UNPROCESSABLE_ENTITY);
    let no_type = admin
        .raw(Method::POST, "/api/files?path=a.txt", &[], TEXT.to_vec())
        .await?;
    assert_eq!(no_type.status, StatusCode::BAD_REQUEST);

    uploaded_id(&admin, "dup.txt").await?;
    assert_eq!(
        upload(&admin, "dup.txt", "text/plain", TEXT).await?.status,
        StatusCode::CONFLICT
    );
    Ok(())
}

#[sqlx::test]
async fn content_streams_with_type_disposition_and_validators(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let id = uploaded_id(&admin, "docs/%EA%B3%84%EC%95%BD%EC%84%9C.txt").await?;
    let reply = admin
        .raw(
            Method::GET,
            &format!("/api/files/{id}/content"),
            &[],
            Vec::new(),
        )
        .await?;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.body.as_ref(), TEXT);
    assert_eq!(reply.header("content-type"), "text/plain");
    assert_eq!(reply.header("content-length"), "20");
    assert_eq!(reply.header("accept-ranges"), "bytes");
    assert_eq!(reply.header("x-content-type-options"), "nosniff");
    let disposition = reply.header("content-disposition");
    assert!(
        disposition.starts_with("inline; filename=\""),
        "{disposition}"
    );
    assert!(
        disposition.contains("filename*=UTF-8''%EA%B3%84%EC%95%BD%EC%84%9C.txt"),
        "{disposition}"
    );

    let forced = admin
        .raw(
            Method::GET,
            &format!("/api/files/{id}/content?download=true"),
            &[],
            Vec::new(),
        )
        .await?;
    assert!(
        forced
            .header("content-disposition")
            .starts_with("attachment;")
    );

    let tag = reply.header("etag").to_owned();
    assert!(tag.starts_with('"') && tag.len() == 66, "{tag}");
    let cached = admin
        .raw(
            Method::GET,
            &format!("/api/files/{id}/content"),
            &[("if-none-match", &tag)],
            Vec::new(),
        )
        .await?;
    assert_eq!(cached.status, StatusCode::NOT_MODIFIED);
    assert!(cached.body.is_empty());
    Ok(())
}

#[sqlx::test]
async fn pdf_viewers_can_read_byte_ranges(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let id = uploaded_id(&admin, "a.txt").await?;
    let url = format!("/api/files/{id}/content");
    let ranged = |range: &'static str| {
        let admin = &admin;
        let url = url.clone();
        async move {
            admin
                .raw(Method::GET, &url, &[("range", range)], Vec::new())
                .await
        }
    };

    let head = ranged("bytes=0-4").await?;
    assert_eq!(head.status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(head.body.as_ref(), b"01234");
    assert_eq!(head.header("content-range"), "bytes 0-4/20");
    assert_eq!(head.header("content-length"), "5");

    let tail = ranged("bytes=-5").await?;
    assert_eq!(tail.status, StatusCode::PARTIAL_CONTENT);
    assert_eq!(tail.body.as_ref(), b"fghij");
    assert_eq!(tail.header("content-range"), "bytes 15-19/20");

    let open = ranged("bytes=18-").await?;
    assert_eq!(open.body.as_ref(), b"ij");
    assert_eq!(open.header("content-range"), "bytes 18-19/20");

    let clamped = ranged("bytes=10-999").await?;
    assert_eq!(clamped.body.as_ref(), b"abcdefghij");
    assert_eq!(clamped.header("content-range"), "bytes 10-19/20");

    let beyond = ranged("bytes=20-").await?;
    assert_eq!(beyond.status, StatusCode::RANGE_NOT_SATISFIABLE);
    assert_eq!(beyond.header("content-range"), "bytes */20");

    let many = ranged("bytes=0-1,4-5").await?;
    assert_eq!(many.status, StatusCode::OK);
    assert_eq!(many.body.as_ref(), TEXT);

    let full = admin.raw(Method::GET, &url, &[], Vec::new()).await?;
    let tag = full.header("etag").to_owned();
    let matching = admin
        .raw(
            Method::GET,
            &url,
            &[("range", "bytes=0-4"), ("if-range", &tag)],
            Vec::new(),
        )
        .await?;
    assert_eq!(matching.status, StatusCode::PARTIAL_CONTENT);
    let changed = admin
        .raw(
            Method::GET,
            &url,
            &[("range", "bytes=0-4"), ("if-range", "\"other\"")],
            Vec::new(),
        )
        .await?;
    assert_eq!(changed.status, StatusCode::OK);
    assert_eq!(changed.body.as_ref(), TEXT);
    Ok(())
}

#[sqlx::test]
async fn deleting_a_file_hides_it_and_frees_the_path(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let id = uploaded_id(&admin, "a.txt").await?;
    assert_eq!(
        admin.delete(&format!("/api/files/{id}")).await?.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        admin.get(&format!("/api/files/{id}")).await?.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        admin.get(&format!("/api/files/{id}/content")).await?.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        admin.delete(&format!("/api/files/{id}")).await?.status,
        StatusCode::NOT_FOUND
    );
    uploaded_id(&admin, "a.txt").await?;
    Ok(())
}

#[sqlx::test]
async fn unknown_files_are_not_found(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    assert_eq!(
        admin.get("/api/files/999").await?.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        admin.get("/api/files?path=nope.txt").await?.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.browser().get("/api/files/1/content").await?.status,
        StatusCode::UNAUTHORIZED
    );
    Ok(())
}
