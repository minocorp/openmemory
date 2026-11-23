mod refresh;

use anyhow::Context;
use memory_api::Problem;
use reqwest::{Method, RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use tokio::sync::Mutex;

use crate::credentials::{Tokens, Vault};
use crate::error::CliError;
use crate::oauth;

const EXPIRY_MARGIN: u64 = 60;

#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub(crate) struct ApiError {
    pub(crate) status: StatusCode,
    pub(crate) problem: Option<Problem>,
    message: String,
}

enum Auth {
    Environment(String),
    Stored { tokens: Mutex<Tokens>, vault: Vault },
}

pub(crate) struct Client {
    http: reqwest::Client,
    host: String,
    auth: Auth,
}

impl Client {
    pub(crate) fn environment(http: reqwest::Client, host: String, token: String) -> Self {
        Self {
            http,
            host,
            auth: Auth::Environment(token),
        }
    }

    pub(crate) fn stored(
        http: reqwest::Client,
        host: String,
        tokens: Tokens,
        vault: Vault,
    ) -> Self {
        Self {
            http,
            host,
            auth: Auth::Stored {
                tokens: Mutex::new(tokens),
                vault,
            },
        }
    }

    pub(crate) fn builder(&self, method: Method, path: &str) -> RequestBuilder {
        self.http.request(method, format!("{}{path}", self.host))
    }

    pub(crate) async fn send(&self, make: impl Fn() -> RequestBuilder) -> anyhow::Result<Response> {
        let token = self.access_token().await?;
        let response = make()
            .bearer_auth(&token)
            .send()
            .await
            .context("reach the server")?;
        if response.status() != StatusCode::UNAUTHORIZED {
            return Ok(response);
        }
        let Auth::Stored { .. } = &self.auth else {
            return Err(CliError::NeedsAuth.into());
        };
        let renewed = self.renew(&token).await?;
        let retried = make()
            .bearer_auth(&renewed)
            .send()
            .await
            .context("reach the server")?;
        if retried.status() == StatusCode::UNAUTHORIZED {
            return Err(CliError::NeedsAuth.into());
        }
        Ok(retried)
    }

    pub(crate) async fn send_once(&self, request: RequestBuilder) -> anyhow::Result<Response> {
        let token = self.access_token().await?;
        let response = request
            .bearer_auth(token)
            .send()
            .await
            .context("reach the server")?;
        if response.status() == StatusCode::UNAUTHORIZED {
            return Err(CliError::NeedsAuth.into());
        }
        Ok(response)
    }

    pub(crate) async fn send_ok(
        &self,
        make: impl Fn() -> RequestBuilder,
    ) -> anyhow::Result<Response> {
        let response = self.send(make).await?;
        check(response).await
    }

    pub(crate) async fn json<T: DeserializeOwned>(
        &self,
        make: impl Fn() -> RequestBuilder,
    ) -> anyhow::Result<T> {
        let response = self.send_ok(make).await?;
        response.json().await.context("read the server's reply")
    }

    async fn access_token(&self) -> anyhow::Result<String> {
        let tokens = match &self.auth {
            Auth::Environment(token) => return Ok(token.clone()),
            Auth::Stored { tokens, .. } => tokens,
        };
        let current = tokens.lock().await.clone();
        let stale = current
            .expires_at
            .is_some_and(|expires_at| expires_at <= oauth::now() + EXPIRY_MARGIN);
        if stale && current.refresh_token.is_some() {
            return self.renew(&current.access_token).await;
        }
        Ok(current.access_token)
    }
}

pub(crate) async fn check(response: Response) -> anyhow::Result<Response> {
    let status = response.status();
    if status.is_success() || status.is_redirection() {
        return Ok(response);
    }
    if status == StatusCode::UNAUTHORIZED {
        return Err(CliError::NeedsAuth.into());
    }
    let bytes = response.bytes().await.unwrap_or_default();
    Err(describe(status, serde_json::from_slice(&bytes).ok()).into())
}

pub(crate) fn describe(status: StatusCode, problem: Option<Problem>) -> ApiError {
    let message = match &problem {
        Some(problem) => match &problem.detail {
            Some(detail) => format!("{}: {detail} ({})", problem.title, problem.code()),
            None => format!("{} ({})", problem.title, problem.code()),
        },
        None => format!("the server answered {status}"),
    };
    ApiError {
        status,
        problem,
        message,
    }
}

#[cfg(test)]
mod tests;
