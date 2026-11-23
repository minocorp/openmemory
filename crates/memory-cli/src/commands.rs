use anyhow::{Context, bail};
use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use memory_ops::{
    DeleteInput, Envelope, HistoryInput, HistoryOutput, MAX_INLINE_UPLOAD_BYTES, MoveInput,
    MoveOutput, ReadInput, ReadOutput, RevertInput, RevertOutput, SearchInput, SearchOutput,
    UploadInput, UploadOutput, WriteInput, WriteOutput,
};
use reqwest::header::CONTENT_TYPE;
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::io::AsyncReadExt;

use crate::cli::{Cli, Command, LoginArgs, UploadArgs, WriteArgs};
use crate::credentials::CredentialsFile;
use crate::server::Server;
use crate::session::{self, Session};
use crate::{device, login, render};

const REVOKE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

pub(crate) async fn run(cli: Cli) -> anyhow::Result<()> {
    let file = CredentialsFile::locate()?;
    let json = cli.json;
    match cli.command {
        Command::Login(args) => sign_in(&file, args).await,
        Command::Logout => sign_out(&file).await,
        Command::Status => status(&file).await,
        Command::Read(args) => {
            tool::<_, Envelope<ReadOutput>>(
                &file,
                json,
                "read",
                &ReadInput::from(args),
                render::read,
            )
            .await
        }
        Command::Search(args) => {
            let input = SearchInput::from(args);
            tool::<_, Envelope<SearchOutput>>(&file, json, "search", &input, render::search).await
        }
        Command::Write(args) => {
            let input = write_input(args).await?;
            tool::<_, WriteOutput>(&file, json, "write", &input, render::written).await
        }
        Command::Move(args) => {
            tool::<_, MoveOutput>(&file, json, "move", &MoveInput::from(args), render::moved).await
        }
        Command::Delete(args) => {
            let input = DeleteInput::from(args);
            tool::<_, WriteOutput>(&file, json, "delete", &input, render::written).await
        }
        Command::History(args) => {
            let input = HistoryInput::from(args);
            tool::<_, Envelope<HistoryOutput>>(&file, json, "history", &input, render::history)
                .await
        }
        Command::Revert(args) => {
            let input = RevertInput::from(args);
            tool::<_, RevertOutput>(&file, json, "revert", &input, render::reverted).await
        }
        Command::Upload(args) => {
            let input = upload_input(args).await?;
            tool::<_, UploadOutput>(&file, json, "upload", &input, render::uploaded).await
        }
    }
}

async fn tool<I: Serialize, O: DeserializeOwned>(
    file: &CredentialsFile,
    json: bool,
    name: &'static str,
    input: &I,
    render: fn(&O) -> String,
) -> anyhow::Result<()> {
    let session = Session::open(file).await?;
    let outcome = session.call::<I, O>(name, input).await;
    session.close().await;
    let reply = outcome?;
    if json {
        println!("{}", reply.raw);
    } else {
        print!("{}", render(&reply.value));
    }
    Ok(())
}

async fn sign_in(file: &CredentialsFile, args: LoginArgs) -> anyhow::Result<()> {
    let server = Server::parse(&args.server)?;
    if args.device {
        device::sign_in(file, &server).await?;
    } else {
        login::browser(file, &server).await?;
    }
    println!("Signed in to {}", server.origin());
    Ok(())
}

async fn status(file: &CredentialsFile) -> anyhow::Result<()> {
    let session = Session::open(file).await?;
    let name = session.server_name().unwrap_or_default();
    session.close().await;
    let saved = file.read().await?.context("credentials disappeared")?;
    println!("server: {}", saved.server);
    println!("software: {name}");
    println!("credentials: {}", file.path().display());
    Ok(())
}

async fn sign_out(file: &CredentialsFile) -> anyhow::Result<()> {
    let Some(saved) = file.read().await? else {
        println!("Not signed in.");
        return Ok(());
    };
    let server = Server::from_origin(saved.server.clone());
    if let Err(error) = revoke(file, &server, &saved.credentials).await {
        eprintln!("could not remove the connection on the server: {error:#}");
    }
    file.remove().await?;
    println!("Signed out of {}", server.origin());
    Ok(())
}

async fn revoke(
    file: &CredentialsFile,
    server: &Server,
    credentials: &rmcp::transport::auth::StoredCredentials,
) -> anyhow::Result<()> {
    let stored = serde_json::to_value(credentials)?;
    let Some(refresh) = stored
        .pointer("/token_response/refresh_token")
        .and_then(serde_json::Value::as_str)
    else {
        bail!("no refresh token is saved");
    };
    let metadata = session::manager(file, server)
        .await?
        .resolve_metadata()
        .await?
        .metadata;
    let endpoint = metadata
        .additional_fields
        .get("revocation_endpoint")
        .and_then(serde_json::Value::as_str)
        .context("the server does not publish a revocation endpoint")?;
    let body = url::form_urlencoded::Serializer::new(String::new())
        .append_pair("token", refresh)
        .append_pair("client_id", &credentials.client_id)
        .finish();
    reqwest::Client::builder()
        .timeout(REVOKE_TIMEOUT)
        .build()?
        .post(endpoint)
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

async fn write_input(mut args: WriteArgs) -> anyhow::Result<WriteInput> {
    let create = text_argument(args.create.take()).await?;
    let overwrite = text_argument(args.overwrite.take()).await?;
    Ok(args.into_input(create, overwrite))
}

async fn text_argument(value: Option<String>) -> anyhow::Result<Option<String>> {
    match value.as_deref() {
        Some("-") => {
            let mut text = String::new();
            tokio::io::stdin()
                .read_to_string(&mut text)
                .await
                .context("read standard input")?;
            Ok(Some(text))
        }
        Some(_) | None => Ok(value),
    }
}

async fn upload_input(args: UploadArgs) -> anyhow::Result<UploadInput> {
    let bytes = tokio::fs::read(&args.file)
        .await
        .with_context(|| format!("read {}", args.file.display()))?;
    let content_base64 = (bytes.len() <= MAX_INLINE_UPLOAD_BYTES).then(|| STANDARD.encode(&bytes));
    Ok(UploadInput {
        path: args.path,
        text: None,
        content_base64,
    })
}
