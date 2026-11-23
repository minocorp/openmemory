use std::path::Path;

use anyhow::Context as _;
use futures_util::StreamExt;
use memory_api::{EntryKind, FileText, FileView};
use reqwest::header::{CONTENT_LENGTH, CONTENT_TYPE};
use reqwest::{Body, Method};
use serde_json::json;
use tokio::io::AsyncWriteExt;
use tokio_util::io::ReaderStream;

use crate::cli::FileCommand;
use crate::client::{Client, check};
use crate::commands::folder;
use crate::context::Context;
use crate::output::note;

pub(super) async fn run(command: FileCommand, context: &Context) -> anyhow::Result<()> {
    match command {
        FileCommand::List { folder } => {
            folder::list(context, folder.as_deref(), EntryKind::File).await
        }
        FileCommand::Upload { local, path } => upload(&local, &path, context).await,
        FileCommand::Download { path, output } => download(&path, output.as_deref(), context).await,
        FileCommand::View { path, page } => view(&path, page, context).await,
    }
}

async fn upload(local: &Path, path: &str, context: &Context) -> anyhow::Result<()> {
    let client = context.client().await?;
    let file = tokio::fs::File::open(local)
        .await
        .with_context(|| format!("open {}", local.display()))?;
    let length = file.metadata().await.context("read the file size")?.len();
    let request = client
        .builder(Method::POST, "/api/files")
        .query(&[("path", path)])
        .header(CONTENT_TYPE, mime_of(local))
        .header(CONTENT_LENGTH, length)
        .body(Body::wrap_stream(ReaderStream::new(file)));
    let response = check(client.send_once(request).await?).await?;
    let stored: FileView = response.json().await.context("read the upload reply")?;
    if context.output.is_json() {
        return context.output.json(&stored);
    }
    note(&format!("Uploaded {} ({} bytes)", stored.path, stored.size));
    Ok(())
}

async fn find(client: &Client, path: &str) -> anyhow::Result<FileView> {
    client
        .json(|| {
            client
                .builder(Method::GET, "/api/files")
                .query(&[("path", path)])
        })
        .await
}

async fn download(path: &str, output: Option<&str>, context: &Context) -> anyhow::Result<()> {
    let client = context.client().await?;
    let file = find(&client, path).await?;
    let url = format!("/api/files/{}/content", file.id);
    let response = client
        .send_ok(|| {
            client
                .builder(Method::GET, &url)
                .query(&[("download", "true")])
        })
        .await?;
    let mut stream = response.bytes_stream();
    let mut written = 0_u64;
    if output == Some("-") {
        let mut stdout = tokio::io::stdout();
        while let Some(chunk) = stream.next().await {
            stdout
                .write_all(&chunk.context("receive the file")?)
                .await
                .context("write stdout")?;
        }
        return stdout.flush().await.context("write stdout");
    }
    let target = output.map_or_else(|| default_name(path), str::to_owned);
    let mut out = tokio::fs::File::create(&target)
        .await
        .with_context(|| format!("create {target}"))?;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk.context("receive the file")?;
        written += u64::try_from(chunk.len()).unwrap_or(0);
        out.write_all(&chunk)
            .await
            .with_context(|| format!("write {target}"))?;
    }
    out.flush()
        .await
        .with_context(|| format!("write {target}"))?;
    if context.output.is_json() {
        return context
            .output
            .json(&json!({ "path": file.path, "saved": target, "bytes": written }));
    }
    note(&format!("Saved {target} ({written} bytes)"));
    Ok(())
}

async fn view(path: &str, page: Option<u32>, context: &Context) -> anyhow::Result<()> {
    let client = context.client().await?;
    let file = find(&client, path).await?;
    if file.page_count == 0 && page.is_none() {
        if context.output.is_json() {
            return context.output.json(&file);
        }
        note(&format!(
            "{} has no text pages ({}, {} bytes)",
            file.path, file.mime, file.size
        ));
        return Ok(());
    }
    let wanted = page.unwrap_or(1).to_string();
    let url = format!("/api/files/{}/text", file.id);
    let text: FileText = client
        .json(|| {
            client
                .builder(Method::GET, &url)
                .query(&[("page", wanted.as_str())])
        })
        .await?;
    if context.output.is_json() {
        return context.output.json(&text);
    }
    print!("{}", text.text);
    note(&format!("Page {} of {}", text.page, text.page_count));
    Ok(())
}

fn default_name(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_owned()
}

fn mime_of(path: &Path) -> &'static str {
    let extension = path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase);
    match extension.as_deref() {
        Some("pdf") => "application/pdf",
        Some("txt") => "text/plain",
        Some("md") => "text/markdown",
        Some("csv") => "text/csv",
        Some("json") => "application/json",
        Some("png") => "image/png",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("m4a") => "audio/mp4",
        Some("xlsx") => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        Some("docx") => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mime_follows_the_extension_and_defaults_to_bytes() {
        assert_eq!(mime_of(Path::new("a/Scan.PDF")), "application/pdf");
        assert_eq!(mime_of(Path::new("a.unknown")), "application/octet-stream");
        assert_eq!(mime_of(Path::new("noext")), "application/octet-stream");
    }

    #[test]
    fn downloads_default_to_the_remote_name() {
        assert_eq!(default_name("scans/2026/a.pdf"), "a.pdf");
        assert_eq!(default_name("a.pdf"), "a.pdf");
    }
}
