use axum::http::{Method, StatusCode};
use sqlx::PgPool;

use crate::support::{App, Browser, Raw, TestResult};

const MOCKUP: &[u8] = b"<h1>Mockup</h1><script>document.title='x'</script>";
const DOCX: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.document";

async fn stored(browser: &Browser, path: &str, mime: &str, bytes: &[u8]) -> TestResult<i64> {
    let reply = browser
        .raw(
            Method::POST,
            &format!("/api/files?path={path}"),
            &[("content-type", mime)],
            bytes.to_vec(),
        )
        .await?;
    assert_eq!(reply.status, StatusCode::CREATED, "{path}");
    let body: serde_json::Value = serde_json::from_slice(&reply.body)?;
    Ok(body["id"].as_i64().ok_or("no id")?)
}

async fn content(browser: &Browser, id: i64, headers: &[(&str, &str)]) -> TestResult<Raw> {
    browser
        .raw(
            Method::GET,
            &format!("/api/files/{id}/content"),
            headers,
            Vec::new(),
        )
        .await
}

#[sqlx::test]
async fn html_runs_sandboxed_in_the_viewer_frame_and_downloads_elsewhere(
    pool: PgPool,
) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let id = stored(&admin, "mockups/home.html", "text/html", MOCKUP).await?;

    let framed = content(&admin, id, &[("sec-fetch-dest", "iframe")]).await?;
    assert_eq!(framed.status, StatusCode::OK);
    assert_eq!(framed.header("content-type"), "text/html");
    assert!(framed.header("content-disposition").starts_with("inline;"));
    assert_eq!(framed.header("x-frame-options"), "SAMEORIGIN");
    assert_eq!(framed.header("vary"), "Sec-Fetch-Dest");
    let policy = framed.header("content-security-policy");
    assert!(policy.starts_with("sandbox allow-scripts;"), "{policy}");
    assert!(!policy.contains("allow-same-origin"), "{policy}");
    assert!(policy.contains("connect-src 'none'"), "{policy}");
    assert!(policy.contains("frame-ancestors 'self'"), "{policy}");

    let opened = content(&admin, id, &[("sec-fetch-dest", "document")]).await?;
    assert!(
        opened
            .header("content-disposition")
            .starts_with("attachment;")
    );
    assert_eq!(opened.header("x-frame-options"), "DENY");
    let unlabeled = content(&admin, id, &[]).await?;
    assert!(
        unlabeled
            .header("content-disposition")
            .starts_with("attachment;")
    );
    Ok(())
}

#[sqlx::test]
async fn svg_shows_without_scripts_and_xml_shows_as_source(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let svg = stored(&admin, "brand/logo.svg", "image/svg+xml", b"<svg/>").await?;
    let logo = content(&admin, svg, &[]).await?;
    assert_eq!(logo.header("content-type"), "image/svg+xml");
    assert!(logo.header("content-disposition").starts_with("inline;"));
    let policy = logo.header("content-security-policy");
    assert!(policy.ends_with("sandbox"), "{policy}");

    let xml = stored(&admin, "tax/invoice.xml", "text/xml", b"<invoice/>").await?;
    let invoice = content(&admin, xml, &[]).await?;
    assert_eq!(invoice.header("content-type"), "text/plain; charset=utf-8");
    assert!(invoice.header("content-disposition").starts_with("inline;"));
    let text = admin.get(&format!("/api/files/{xml}/text")).await?;
    assert_eq!(text.body["text"], "<invoice/>");
    Ok(())
}

#[sqlx::test]
async fn untyped_uploads_take_the_type_of_their_extension(pool: PgPool) -> TestResult {
    let app = App::new(pool);
    let (admin, _) = app.admin().await?;
    let id = stored(
        &admin,
        "contracts/acme.docx",
        "application/octet-stream",
        b"PK",
    )
    .await?;
    let file = admin.get(&format!("/api/files/{id}")).await?;
    assert_eq!(file.body["mime"], DOCX);
    assert_eq!(file.body["page_count"], 0);
    let document = content(&admin, id, &[]).await?;
    assert!(
        document
            .header("content-disposition")
            .starts_with("attachment;")
    );

    let unknown = admin
        .raw(
            Method::POST,
            "/api/files?path=a.bin",
            &[("content-type", "application/octet-stream")],
            b"x".to_vec(),
        )
        .await?;
    assert_eq!(unknown.status, StatusCode::UNSUPPORTED_MEDIA_TYPE);
    Ok(())
}
