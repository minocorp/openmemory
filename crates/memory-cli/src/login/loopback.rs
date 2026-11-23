use std::time::Duration;

use anyhow::Context;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use url::Url;

use crate::error::CliError;

const CALLBACK_PATH: &str = "/callback";
const MAX_REQUEST_BYTES: usize = 8192;
const READ_TIMEOUT: Duration = Duration::from_secs(5);
const DONE_PAGE: &str =
    "<!doctype html><title>memory</title><p>You can close this tab and return to the terminal.";
const FAILED_PAGE: &str =
    "<!doctype html><title>memory</title><p>Sign-in failed. Return to the terminal.";

#[derive(Debug, PartialEq, Eq)]
enum Callback {
    Code(String),
    Denied,
    Mismatch,
    Elsewhere,
}

pub(crate) struct Loopback {
    listener: TcpListener,
    port: u16,
}

impl Loopback {
    pub(crate) async fn bind() -> anyhow::Result<Self> {
        let listener = TcpListener::bind(("127.0.0.1", 0))
            .await
            .context("listen on a loopback port for the sign-in redirect")?;
        let port = listener
            .local_addr()
            .context("read the loopback port")?
            .port();
        Ok(Self { listener, port })
    }

    pub(crate) fn redirect_uri(&self) -> String {
        format!("http://127.0.0.1:{}{CALLBACK_PATH}", self.port)
    }

    pub(crate) async fn receive(self, state: &str, issuer: &str) -> anyhow::Result<String> {
        loop {
            let (mut stream, _) = self
                .listener
                .accept()
                .await
                .context("accept the redirect")?;
            let Some(target) = read_target(&mut stream).await else {
                continue;
            };
            match classify(&target, state, issuer) {
                Callback::Code(code) => {
                    respond(&mut stream, "200 OK", DONE_PAGE).await;
                    return Ok(code);
                }
                Callback::Denied => {
                    respond(&mut stream, "200 OK", FAILED_PAGE).await;
                    return Err(CliError::Cancelled.into());
                }
                Callback::Mismatch => {
                    respond(&mut stream, "400 Bad Request", FAILED_PAGE).await;
                    anyhow::bail!("the sign-in redirect did not match this login attempt");
                }
                Callback::Elsewhere => respond(&mut stream, "404 Not Found", "").await,
            }
        }
    }
}

async fn read_target(stream: &mut TcpStream) -> Option<String> {
    let mut buffer = Vec::new();
    let mut chunk = [0_u8; 1024];
    while !buffer.windows(2).any(|pair| pair == b"\r\n") && buffer.len() < MAX_REQUEST_BYTES {
        let read = tokio::time::timeout(READ_TIMEOUT, stream.read(&mut chunk))
            .await
            .ok()?
            .ok()?;
        if read == 0 {
            break;
        }
        buffer.extend_from_slice(chunk.get(..read)?);
    }
    let text = String::from_utf8_lossy(&buffer);
    let line = text.lines().next()?;
    let mut parts = line.split(' ');
    (parts.next()? == "GET").then(|| parts.next().map(str::to_owned))?
}

async fn respond(stream: &mut TcpStream, status: &str, page: &str) {
    let reply = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{page}",
        page.len()
    );
    stream.write_all(reply.as_bytes()).await.ok();
    stream.shutdown().await.ok();
}

fn classify(target: &str, state: &str, issuer: &str) -> Callback {
    let Ok(url) = Url::parse(&format!("http://127.0.0.1{target}")) else {
        return Callback::Elsewhere;
    };
    if url.path() != CALLBACK_PATH {
        return Callback::Elsewhere;
    }
    let pair = |name: &str| {
        url.query_pairs()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.into_owned())
    };
    if pair("state").as_deref() != Some(state) || pair("iss").as_deref() != Some(issuer) {
        return Callback::Mismatch;
    }
    match (pair("code"), pair("error")) {
        (Some(code), None) => Callback::Code(code),
        (_, Some(error)) if error == "access_denied" => Callback::Denied,
        _ => Callback::Mismatch,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ISSUER: &str = "https://m.example";

    #[test]
    fn a_matching_redirect_yields_the_code() {
        let target = "/callback?code=abc&state=s1&iss=https%3A%2F%2Fm.example";
        assert_eq!(
            classify(target, "s1", ISSUER),
            Callback::Code("abc".to_owned())
        );
    }

    #[test]
    fn a_wrong_state_or_issuer_is_rejected() {
        assert_eq!(
            classify(
                "/callback?code=abc&state=other&iss=https%3A%2F%2Fm.example",
                "s1",
                ISSUER
            ),
            Callback::Mismatch
        );
        assert_eq!(
            classify(
                "/callback?code=abc&state=s1&iss=https%3A%2F%2Fevil.example",
                "s1",
                ISSUER
            ),
            Callback::Mismatch
        );
        assert_eq!(
            classify("/callback?code=abc&state=s1", "s1", ISSUER),
            Callback::Mismatch
        );
    }

    #[test]
    fn a_denial_and_stray_requests_are_told_apart() {
        let denied = "/callback?error=access_denied&state=s1&iss=https%3A%2F%2Fm.example";
        assert_eq!(classify(denied, "s1", ISSUER), Callback::Denied);
        assert_eq!(classify("/favicon.ico", "s1", ISSUER), Callback::Elsewhere);
    }

    #[tokio::test]
    async fn the_listener_hands_back_the_code_and_ignores_strays() {
        let loopback = Loopback::bind().await.unwrap();
        let base = loopback.redirect_uri();
        let waiting = tokio::spawn(loopback.receive("s1", ISSUER));
        let http = reqwest::Client::new();
        http.get(base.replace("/callback", "/favicon.ico"))
            .send()
            .await
            .unwrap();
        let reply = http
            .get(format!(
                "{base}?code=abc&state=s1&iss=https%3A%2F%2Fm.example"
            ))
            .send()
            .await
            .unwrap();
        assert!(reply.status().is_success());
        assert_eq!(waiting.await.unwrap().unwrap(), "abc");
    }
}
