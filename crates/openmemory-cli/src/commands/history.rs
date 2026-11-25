use openmemory_api::{Changes, IF_MATCH, Restored, SeenRequest, SeenState, VersionList};
use reqwest::Method;

use crate::commands::page::lookup;
use crate::context::Context;
use crate::output::note;
use crate::render;

const RECENT_GROUPS: &str = "20";

pub(super) async fn history(path: Option<&str>, context: &Context) -> anyhow::Result<()> {
    let client = context.client().await?;
    let Some(path) = path else {
        let recent: Changes = client
            .json(|| {
                client
                    .builder(Method::GET, "/api/changes")
                    .query(&[("since", "0"), ("limit", RECENT_GROUPS)])
            })
            .await?;
        return show_changes(context, &recent);
    };
    let found = lookup(&client, path).await?;
    let url = format!("/api/pages/{}/versions", found.page.id);
    let list: VersionList = client.json(|| client.builder(Method::GET, &url)).await?;
    if context.output.is_json() {
        return context.output.json(&list);
    }
    print!("{}", render::versions(&list, context.output));
    if list.next.is_some() {
        note("Older versions exist: page through them with `openmemory api`");
    }
    Ok(())
}

pub(super) async fn revert(path: &str, version: i32, context: &Context) -> anyhow::Result<()> {
    let client = context.client().await?;
    let found = lookup(&client, path).await?;
    let url = format!("/api/pages/{}/versions/{version}/restore", found.page.id);
    let restored: Restored = client
        .json(|| {
            client
                .builder(Method::POST, &url)
                .header(IF_MATCH, &found.etag)
        })
        .await?;
    if context.output.is_json() {
        return context.output.json(&restored);
    }
    for written in &restored.reverted {
        note(&format!(
            "Restored {} to version {version} as version {}",
            written.path, written.version
        ));
    }
    Ok(())
}

pub(super) async fn changes(seen: bool, context: &Context) -> anyhow::Result<()> {
    let client = context.client().await?;
    let found: Changes = client
        .json(|| client.builder(Method::GET, "/api/changes"))
        .await?;
    show_changes(context, &found)?;
    if seen && found.last_seq > found.since {
        let request = SeenRequest {
            seq: found.last_seq,
        };
        let _state: SeenState = client
            .json(|| {
                client
                    .builder(Method::POST, "/api/changes/seen")
                    .json(&request)
            })
            .await?;
        note("Marked as seen");
    }
    Ok(())
}

fn show_changes(context: &Context, found: &Changes) -> anyhow::Result<()> {
    if context.output.is_json() {
        return context.output.json(found);
    }
    print!("{}", render::changes(found, context.output));
    Ok(())
}
