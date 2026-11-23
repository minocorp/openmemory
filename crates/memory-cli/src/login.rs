use std::io::IsTerminal;
use std::time::Duration;

use anyhow::{Context, anyhow, bail};
use rmcp::transport::auth::{AuthorizationRequest, AuthorizationSession};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use url::Url;

use crate::credentials::CredentialsFile;
use crate::server::Server;
use crate::session;

const CLIENT_NAME: &str = "memory CLI";
const CALLBACK_PATH: &str = "/callback";
const CALLBACK_TIMEOUT: Duration = Duration::from_secs(300);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(10);
const MAX_REQUEST_BYTES: usize = 8192;
const DONE_PAGE: &str = "<!doctype html><title>memory</title><p>Signed in. You can close this tab.";

pub(crate) async fn browser(file: &CredentialsFile, server: &Server) -> anyhow::Result<()> {
    let listener = TcpListener::bind(("127.0.0.1", 0))
        .await
        .context("listen on a loopback port for the sign-in redirect")?;
    let redirect = format!(
        "http://127.0.0.1:{}{CALLBACK_PATH}",
        listener.local_addr()?.port()
    );
    let mut manager = session::manager(file, server).await?;
    let metadata = manager
        .resolve_metadata()
        .await
        .context("discover how the server signs in")?
        .metadata;
    manager.set_metadata(metadata);
    let request = AuthorizationRequest::new(&redirect).with_client_name(CLIENT_NAME);
    let authorization = AuthorizationSession::new(manager, request)
        .await
        .map_err(|(_, error)| anyhow!(error).context("register with the server"))?;
    open_or_print(authorization.get_authorization_url());
    let callback = tokio::time::timeout(CALLBACK_TIMEOUT, wait_for_redirect(&listener, &redirect))
        .await
        .map_err(|_| anyhow!("no sign-in arrived within five minutes"))??;
    authorization
        .handle_callback_url(&callback)
        .await
        .context("finish signing in")?;
    Ok(())
}

fn open_or_print(url: &str) {
    println!("Open this address to sign in:\n{url}");
    if !(std::io::stdin().is_terminal() && std::io::stdout().is_terminal()) {
        return;
    }
    let opener = if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    let _ = std::process::Command::new(opener)
        .arg(url)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();
}

async fn wait_for_redirect(listener: &TcpListener, redirect: &str) -> anyhow::Result<String> {
    let base = Url::parse(redirect)?;
    loop {
        let (stream, _) = listener
            .accept()
            .await
            .context("accept the sign-in redirect")?;
        let Ok(Ok(target)) = tokio::time::timeout(REQUEST_TIMEOUT, read_target(stream)).await
        else {
            continue;
        };
        let (stream, target) = target;
        let url = base.join(&target)?;
        if url.path() != CALLBACK_PATH {
            respond(stream, "404 Not Found", "").await;
            continue;
        }
        let failure = url
            .query_pairs()
            .find(|(key, _)| key == "error")
            .map(|(_, value)| value.into_owned());
        respond(stream, "200 OK", DONE_PAGE).await;
        if let Some(reason) = failure {
            bail!("the server refused the sign-in: {reason}");
        }
        return Ok(url.into());
    }
}

async fn read_target(mut stream: TcpStream) -> std::io::Result<(TcpStream, String)> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];
    while !buffer.windows(2).any(|pair| pair == b"\r\n") && buffer.len() < MAX_REQUEST_BYTES {
        let read = stream.read(&mut chunk).await?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(chunk.get(..read).unwrap_or_default());
    }
    let line = String::from_utf8_lossy(&buffer)
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned();
    let target = line
        .strip_prefix("GET ")
        .and_then(|rest| rest.split(' ').next())
        .map(str::to_owned)
        .ok_or_else(|| std::io::Error::other("not a GET request"))?;
    Ok((stream, target))
}

async fn respond(mut stream: TcpStream, status: &str, body: &str) {
    let reply = format!(
        "HTTP/1.1 {status}\r\ncontent-type: text/html; charset=utf-8\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(reply.as_bytes()).await;
    let _ = stream.shutdown().await;
}
