use axum::extract::{Query, State};
use axum::response::{IntoResponse, Redirect, Response};
use serde::Deserialize;
use url::Url;

use crate::auth::token::Secret;
use crate::oauth::clients;
use crate::oauth::error::{OAuthError, OAuthErrorCode};
use crate::oauth::pkce::CodeChallenge;
use crate::oauth::queries::consents::{self, NewConsent};
use crate::oauth::redirect;
use crate::state::AppState;

const CONSENT_SECONDS: f64 = 600.0;
const MAX_STATE_LENGTH: usize = 1024;

#[derive(Debug, Deserialize)]
pub(super) struct AuthorizeQuery {
    response_type: Option<String>,
    client_id: Option<String>,
    redirect_uri: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
    state: Option<String>,
    resource: Option<String>,
}

pub(super) async fn authorize(
    State(state): State<AppState>,
    Query(query): Query<AuthorizeQuery>,
) -> Result<Response, OAuthError> {
    let client_id = query
        .client_id
        .as_deref()
        .ok_or(OAuthErrorCode::InvalidRequest)?;
    let client = clients::resolve(&state, client_id).await?;
    let redirect_uri = query
        .redirect_uri
        .as_deref()
        .filter(|uri| redirect::matches(&client.redirect_uris, uri))
        .ok_or(OAuthErrorCode::InvalidRequest)?;
    let client_state = query.state.as_deref();
    let failure =
        |code: OAuthErrorCode| Ok(error_redirect(&state, redirect_uri, client_state, code));
    if client_state.is_some_and(|value| value.len() > MAX_STATE_LENGTH) {
        return failure(OAuthErrorCode::InvalidRequest);
    }
    if query.response_type.as_deref() != Some("code") {
        return failure(OAuthErrorCode::InvalidRequest);
    }
    let challenge = query
        .code_challenge
        .as_deref()
        .and_then(CodeChallenge::parse);
    let Some(challenge) =
        challenge.filter(|_| query.code_challenge_method.as_deref() == Some("S256"))
    else {
        return failure(OAuthErrorCode::InvalidRequest);
    };
    let Some(audience) = query
        .resource
        .as_deref()
        .and_then(|resource| state.public_url().audience_of(resource))
    else {
        return failure(OAuthErrorCode::InvalidTarget);
    };
    let token = Secret::generate();
    let mut conn = state.pool().acquire().await?;
    consents::delete_expired(&mut conn).await?;
    consents::insert(
        &mut conn,
        &NewConsent {
            token: &token.hash(),
            oauth_client_id: client.id,
            redirect_uri,
            code_challenge: challenge.as_str(),
            client_state,
            audience,
            lifetime_seconds: CONSENT_SECONDS,
        },
    )
    .await?;
    let approve = state
        .public_url()
        .join(&format!("/approve?consent={}", token.expose()));
    Ok(Redirect::to(&approve).into_response())
}

pub(super) fn error_redirect(
    state: &AppState,
    redirect_uri: &str,
    client_state: Option<&str>,
    code: OAuthErrorCode,
) -> Response {
    redirect_with(
        state,
        redirect_uri,
        client_state,
        &[("error", code.as_str())],
    )
}

pub(super) fn redirect_with(
    state: &AppState,
    redirect_uri: &str,
    client_state: Option<&str>,
    parameters: &[(&str, &str)],
) -> Response {
    Redirect::to(&redirect_target(
        state,
        redirect_uri,
        client_state,
        parameters,
    ))
    .into_response()
}

pub(super) fn redirect_target(
    state: &AppState,
    redirect_uri: &str,
    client_state: Option<&str>,
    parameters: &[(&str, &str)],
) -> String {
    let Ok(mut url) = Url::parse(redirect_uri) else {
        return redirect_uri.to_owned();
    };
    {
        let mut query = url.query_pairs_mut();
        for (key, value) in parameters {
            query.append_pair(key, value);
        }
        if let Some(value) = client_state {
            query.append_pair("state", value);
        }
        query.append_pair("iss", state.public_url().issuer());
    }
    url.into()
}
