use std::path::{Path, PathBuf};

use anyhow::Context as _;
use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use openmemory_api::{IF_MATCH, PageOverwrite, PageView};
use reqwest::{Method, StatusCode};

use crate::commands::page::{self, lookup};
use crate::context::Context;
use crate::error::CliError;
use crate::output::note;

pub(super) async fn run(path: &str, context: &Context) -> anyhow::Result<()> {
    if context.no_input {
        return Err(CliError::Usage(
            "`page edit` opens an editor: use `openmemory page append`, or `openmemory page create`, with --no-input".to_owned(),
        )
        .into());
    }
    let client = context.client().await?;
    let found = lookup(&client, path).await?;
    let draft = draft_path();
    tokio::fs::write(&draft, &found.page.content)
        .await
        .context("write the editing copy")?;
    let outcome = open_editor(context.editor.as_deref(), &draft).await;
    let text = match outcome {
        Ok(()) => tokio::fs::read_to_string(&draft)
            .await
            .context("read the editing copy"),
        Err(error) => Err(error),
    };
    let text = match text {
        Ok(text) => text,
        Err(error) => {
            tokio::fs::remove_file(&draft).await.ok();
            return Err(error);
        }
    };
    if text == found.page.content {
        tokio::fs::remove_file(&draft).await.ok();
        note("No changes");
        return Ok(());
    }
    let body = PageOverwrite {
        content: text,
        task: None,
    };
    let url = format!("/api/pages/{}", found.page.id);
    let response = client
        .send(|| {
            client
                .builder(Method::PUT, &url)
                .header(IF_MATCH, &found.etag)
                .json(&body)
        })
        .await?;
    if response.status() == StatusCode::PRECONDITION_FAILED {
        return Err(anyhow::anyhow!(
            "someone changed {path} while you were editing: your edit is kept in {}; run `openmemory page edit {path}` again and apply it",
            draft.display()
        ));
    }
    let response = crate::client::check(response).await?;
    let updated: PageView = response.json().await.context("read the saved page")?;
    tokio::fs::remove_file(&draft).await.ok();
    page::saved(context, &updated, "Saved")
}

fn draft_path() -> PathBuf {
    let mut bytes = [0_u8; 6];
    rand::fill(&mut bytes);
    let suffix = URL_SAFE_NO_PAD.encode(bytes);
    std::env::temp_dir().join(format!("openmemory-edit-{suffix}.md"))
}

async fn open_editor(editor: Option<&str>, file: &Path) -> anyhow::Result<()> {
    let command = editor.unwrap_or("vi");
    let mut words = command.split_whitespace();
    let program = words.next().unwrap_or("vi");
    let status = tokio::process::Command::new(program)
        .args(words)
        .arg(file)
        .kill_on_drop(true)
        .status()
        .await
        .with_context(|| format!("start the editor `{program}`: set $VISUAL or $EDITOR"))?;
    if status.success() {
        Ok(())
    } else {
        Err(CliError::Cancelled.into())
    }
}
