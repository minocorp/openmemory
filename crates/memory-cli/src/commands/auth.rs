use memory_api::FolderListing;
use reqwest::Method;
use serde_json::json;

use crate::cli::AuthCommand;
use crate::context::Context;
use crate::credentials::{Saved, Tokens};
use crate::error::CliError;
use crate::login;
use crate::oauth;
use crate::output::note;

pub(super) async fn run(command: AuthCommand, context: &Context) -> anyhow::Result<()> {
    match command {
        AuthCommand::Login { device } => sign_in(device, context).await,
        AuthCommand::Status => status(context).await,
        AuthCommand::Logout => sign_out(context).await,
    }
}

async fn sign_in(device: bool, context: &Context) -> anyhow::Result<()> {
    let host = context.host().await?;
    let endpoints = oauth::discover(&context.http, &host).await?;
    let flow = async {
        if device {
            login::device(&context.http, &host, &endpoints).await
        } else {
            login::browser(&context.http, &host, &endpoints, open_browser).await
        }
    };
    let tokens = tokio::select! {
        tokens = flow => tokens?,
        _ = tokio::signal::ctrl_c() => return Err(CliError::Cancelled.into()),
    };
    let saved = context.vault.save(&host, &tokens).await?;
    context.vault.remember_host(&host).await?;
    if context.output.is_json() {
        return context
            .output
            .json(&json!({ "host": host, "authenticated": true }));
    }
    let place = match saved {
        Saved::Keychain => "the system keychain",
        Saved::File => "a private file: no keychain is available",
    };
    note(&format!("Signed in to {host}; credentials are in {place}"));
    Ok(())
}

async fn status(context: &Context) -> anyhow::Result<()> {
    let host = context.host().await?;
    let (source, tokens) = match &context.token {
        Some(_) => ("environment", None),
        None => ("stored", context.vault.load(&host).await?),
    };
    let known = context.token.is_some() || tokens.is_some();
    let valid = known && still_valid(context).await?;
    let expires_at = tokens.as_ref().and_then(|tokens| tokens.expires_at);
    if context.output.is_json() {
        context.output.json(&json!({
            "host": host,
            "authenticated": valid,
            "source": source,
            "expires_at": expires_at,
        }))?;
    } else {
        print_status(context, &host, valid, source, tokens.as_ref());
    }
    if valid {
        Ok(())
    } else {
        Err(CliError::NeedsAuth.into())
    }
}

fn print_status(context: &Context, host: &str, valid: bool, source: &str, tokens: Option<&Tokens>) {
    println!("host: {host}");
    println!("authenticated: {}", if valid { "yes" } else { "no" });
    if valid {
        println!("credentials: {source}");
    }
    if let Some(expires_at) = tokens.and_then(|tokens| tokens.expires_at) {
        let remaining = expires_at.saturating_sub(oauth::now());
        println!("access token expires in: {remaining}s");
    }
    if !valid {
        note(
            &context
                .output
                .red("not signed in: run `memory auth login` or set MEMORY_TOKEN"),
        );
    }
}

async fn still_valid(context: &Context) -> anyhow::Result<bool> {
    let client = context.client().await?;
    let probe = client
        .json::<FolderListing>(|| {
            client
                .builder(Method::GET, "/api/folder")
                .query(&[("path", ""), ("limit", "1")])
        })
        .await;
    match probe {
        Ok(_) => Ok(true),
        Err(error) if matches!(error.downcast_ref::<CliError>(), Some(CliError::NeedsAuth)) => {
            Ok(false)
        }
        Err(error) => Err(error),
    }
}

async fn sign_out(context: &Context) -> anyhow::Result<()> {
    let host = context.host().await?;
    if let Some(tokens) = context.vault.load(&host).await? {
        if let Ok(endpoints) = oauth::discover(&context.http, &host).await {
            let token = tokens
                .refresh_token
                .as_deref()
                .unwrap_or(&tokens.access_token);
            oauth::revoke(&context.http, &endpoints, token).await;
        }
        context.vault.delete(&host).await?;
    }
    if context.output.is_json() {
        return context
            .output
            .json(&json!({ "host": host, "authenticated": false }));
    }
    note(&format!("Signed out of {host}"));
    if context.token.is_some() {
        note("MEMORY_TOKEN is still set in this environment");
    }
    Ok(())
}

fn open_browser(url: &str) {
    let (program, arguments): (&str, Vec<&str>) = if cfg!(target_os = "macos") {
        ("open", vec![url])
    } else if cfg!(windows) {
        ("cmd", vec!["/c", "start", "", url])
    } else {
        ("xdg-open", vec![url])
    };
    tokio::process::Command::new(program)
        .args(arguments)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .ok();
}
