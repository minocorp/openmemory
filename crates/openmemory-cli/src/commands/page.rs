use std::io::IsTerminal;

use anyhow::Context as _;
use openmemory_api::{
    AppendBody, ETAG, EntryKind, IF_MATCH, IF_NONE_MATCH, Moved, PageCreate, PageEdit, PageMove,
    PageView,
};
use reqwest::Method;
use serde_json::json;
use tokio::io::AsyncReadExt;

use crate::cli::PageCommand;
use crate::client::Client;
use crate::commands::{edit, folder};
use crate::context::Context;
use crate::error::CliError;
use crate::output::note;

pub(super) struct Found {
    pub(super) page: PageView,
    pub(super) etag: String,
}

pub(super) async fn run(command: PageCommand, context: &Context) -> anyhow::Result<()> {
    match command {
        PageCommand::List { folder } => {
            folder::list(context, folder.as_deref(), EntryKind::Page).await
        }
        PageCommand::View { path } => view(&path, context).await,
        PageCommand::Create { path, source } => create(&path, source.as_deref(), context).await,
        PageCommand::Edit { path } => edit::run(&path, context).await,
        PageCommand::Append {
            path,
            text,
            source,
            date,
        } => {
            let body = AppendBody { text, source, date };
            append(&path, body, context).await
        }
        PageCommand::Move { from, to } => relocate(&from, &to, context).await,
        PageCommand::Delete { path } => delete(&path, context).await,
    }
}

pub(super) async fn lookup(client: &Client, path: &str) -> anyhow::Result<Found> {
    let response = client
        .send_ok(|| {
            client
                .builder(Method::GET, "/api/pages")
                .query(&[("path", path)])
        })
        .await?;
    let etag = response
        .headers()
        .get(ETAG)
        .and_then(|value| value.to_str().ok())
        .context("the server sent no ETag")?
        .to_owned();
    let page = response.json().await.context("read the page")?;
    Ok(Found { page, etag })
}

async fn view(path: &str, context: &Context) -> anyhow::Result<()> {
    let client = context.client().await?;
    let found = lookup(&client, path).await?;
    if context.output.is_json() {
        return context.output.json(&found.page);
    }
    print!("{}", found.page.content);
    Ok(())
}

async fn create(path: &str, source: Option<&str>, context: &Context) -> anyhow::Result<()> {
    let markdown = read_source(source).await?;
    let client = context.client().await?;
    let body = PageCreate {
        content: markdown,
        task: None,
    };
    let created: PageView = client
        .json(|| {
            client
                .builder(Method::PUT, "/api/pages")
                .query(&[("path", path)])
                .header(IF_NONE_MATCH, "*")
                .json(&body)
        })
        .await?;
    saved(context, &created, "Created")
}

async fn read_source(source: Option<&str>) -> anyhow::Result<String> {
    match source {
        Some(file) if file != "-" => tokio::fs::read_to_string(file)
            .await
            .with_context(|| format!("read {file}")),
        _ => {
            if source.is_none() && std::io::stdin().is_terminal() {
                return Err(CliError::Usage(
                    "no content: pass a markdown file or pipe it to stdin".to_owned(),
                )
                .into());
            }
            let mut text = String::new();
            tokio::io::stdin()
                .read_to_string(&mut text)
                .await
                .context("read stdin")?;
            Ok(text)
        }
    }
}

async fn append(path: &str, body: AppendBody, context: &Context) -> anyhow::Result<()> {
    let client = context.client().await?;
    let found = lookup(&client, path).await?;
    let edit = PageEdit {
        replace: None,
        append: Some(body),
        task: None,
    };
    let url = format!("/api/pages/{}", found.page.id);
    let updated: PageView = client
        .json(|| {
            client
                .builder(Method::PATCH, &url)
                .header(IF_MATCH, &found.etag)
                .json(&edit)
        })
        .await?;
    saved(context, &updated, "Appended to")
}

async fn relocate(from: &str, to: &str, context: &Context) -> anyhow::Result<()> {
    let client = context.client().await?;
    let found = lookup(&client, from).await?;
    let body = PageMove {
        to: to.to_owned(),
        task: None,
    };
    let url = format!("/api/pages/{}/move", found.page.id);
    let moved: Moved = client
        .json(|| {
            client
                .builder(Method::POST, &url)
                .header(IF_MATCH, &found.etag)
                .json(&body)
        })
        .await?;
    if context.output.is_json() {
        return context.output.json(&moved);
    }
    note(&format!("Moved {from} to {}", moved.path));
    Ok(())
}

async fn delete(path: &str, context: &Context) -> anyhow::Result<()> {
    let client = context.client().await?;
    let found = lookup(&client, path).await?;
    let url = format!("/api/pages/{}", found.page.id);
    client
        .send_ok(|| {
            client
                .builder(Method::DELETE, &url)
                .header(IF_MATCH, &found.etag)
        })
        .await?;
    if context.output.is_json() {
        return context.output.json(&json!({ "deleted": found.page.path }));
    }
    note(&format!("Deleted {}", found.page.path));
    Ok(())
}

pub(super) fn saved(context: &Context, page: &PageView, verb: &str) -> anyhow::Result<()> {
    if context.output.is_json() {
        return context.output.json(page);
    }
    note(&format!("{verb} {} (version {})", page.path, page.version));
    Ok(())
}
