use axum::extract::{Path, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use url::Url;
use utoipa::{OpenApi, ToSchema};
use webauthn_rs::prelude::{PublicKeyCredential, RequestChallengeResponse};

use crate::access::{Audience, Folders, Name, connections};
use crate::auth::ceremony::{self, Assertion};
use crate::auth::token::Secret;
use crate::auth::{AuthError, WebSession};
use crate::oauth::authorize::redirect_target;
use crate::oauth::clients::{self, Client};
use crate::oauth::device::UserCode;
use crate::oauth::error::OAuthErrorCode;
use crate::oauth::queries::consents::{self, ConsentRow};
use crate::oauth::queries::{codes, devices};
use crate::oauth::redirect;
use crate::state::AppState;
use memory_api::Problem;

const CODE_SECONDS: f64 = 60.0;
const MIN_LABEL_CHARS: usize = 3;
const GENERIC_SECOND_LEVEL: [&str; 8] = ["co", "com", "org", "net", "gov", "edu", "ac", "or"];

#[derive(Debug)]
enum Pending {
    Authorization(ConsentRow),
    Device(UserCode),
}

#[derive(Debug, Serialize, ToSchema)]
struct ClientSummary {
    client_id: String,
    name: String,
    unverified: bool,
    official: bool,
}

#[derive(Debug, Serialize, ToSchema)]
struct ConsentView {
    kind: &'static str,
    audience: Audience,
    client: ClientSummary,
    redirect_host: Option<String>,
    localhost_only: bool,
    name_matches_redirect: Option<bool>,
    proposed_name: String,
    default_scopes: Vec<String>,
    default_can_write: bool,
}

#[derive(Debug, Serialize, ToSchema)]
struct ChallengeView {
    challenge: String,
    #[schema(value_type = Object)]
    options: RequestChallengeResponse,
}

#[derive(Debug, Deserialize, ToSchema)]
struct ApproveRequest {
    challenge: String,
    #[schema(value_type = Object)]
    credential: PublicKeyCredential,
    name: Option<Name>,
    scopes: Option<Vec<String>>,
    can_write: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
struct Decision {
    redirect_uri: Option<String>,
}

#[derive(OpenApi)]
#[openapi(
    paths(view, challenge, approve, deny),
    components(schemas(ClientSummary, ConsentView, ChallengeView, ApproveRequest, Decision))
)]
pub(super) struct Doc;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/oauth/consents/{reference}", get(view))
        .route("/api/oauth/consents/{reference}/challenge", post(challenge))
        .route("/api/oauth/consents/{reference}/approve", post(approve))
        .route("/api/oauth/consents/{reference}/deny", post(deny))
}

#[utoipa::path(
    get,
    path = "/api/oauth/consents/{reference}",
    tag = "oauth",
    security(("session" = [])),
    params(("reference" = String, Path, description = "Consent token or device user code")),
    responses(
        (status = 200, body = ConsentView),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 404, body = Problem, content_type = "application/problem+json"),
    )
)]
async fn view(
    State(state): State<AppState>,
    session: WebSession,
    Path(reference): Path<String>,
) -> Result<Json<ConsentView>, AuthError> {
    let mut conn = state.pool().acquire().await?;
    let (pending, client, audience) = lookup(&mut conn, &reference).await?;
    let proposed = connections::proposed_name(&mut conn, session.user_id(), &client.name).await?;
    let (kind, redirect_uri) = match &pending {
        Pending::Authorization(row) => ("authorization", Some(row.redirect_uri.as_str())),
        Pending::Device(_) => ("device", None),
    };
    let redirect = redirect_uri.and_then(|uri| Url::parse(uri).ok());
    let redirect_host = redirect
        .as_ref()
        .and_then(|url| url.host_str().map(str::to_owned));
    let localhost_only = redirect.as_ref().is_some_and(redirect::is_loopback);
    let name_matches_redirect = redirect_host
        .as_deref()
        .filter(|_| !localhost_only)
        .map(|host| name_matches_host(&client.name, host));
    Ok(Json(ConsentView {
        kind,
        audience,
        client: ClientSummary {
            unverified: client.is_dynamic(),
            official: client.is_official(),
            client_id: client.public_id,
            name: client.name,
        },
        redirect_host,
        localhost_only,
        name_matches_redirect,
        proposed_name: proposed.to_string(),
        default_scopes: connections::scopes::encode(&Folders::All),
        default_can_write: true,
    }))
}

fn name_matches_host(name: &str, host: &str) -> bool {
    let name = alphanumeric(name);
    let mut labels: Vec<&str> = host.split('.').collect();
    labels.pop();
    if labels.len() > 1
        && labels
            .last()
            .is_some_and(|label| GENERIC_SECOND_LEVEL.contains(label))
    {
        labels.pop();
    }
    labels.last().is_some_and(|label| {
        let label = alphanumeric(label);
        label.chars().count() >= MIN_LABEL_CHARS && name.contains(&label)
    })
}

fn alphanumeric(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

#[utoipa::path(
    post,
    path = "/api/oauth/consents/{reference}/challenge",
    tag = "oauth",
    security(("session" = [])),
    params(("reference" = String, Path, description = "Consent token or device user code")),
    responses(
        (status = 200, body = ChallengeView),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 404, body = Problem, content_type = "application/problem+json"),
    )
)]
async fn challenge(
    State(state): State<AppState>,
    session: WebSession,
    Path(reference): Path<String>,
) -> Result<Json<ChallengeView>, AuthError> {
    let mut conn = state.pool().acquire().await?;
    lookup(&mut conn, &reference).await?;
    drop(conn);
    let (challenge, options) =
        ceremony::start_authentication(&state, session.user_id(), Assertion::Approval).await?;
    Ok(Json(ChallengeView {
        challenge: challenge.expose().to_owned(),
        options,
    }))
}

#[utoipa::path(
    post,
    path = "/api/oauth/consents/{reference}/approve",
    tag = "oauth",
    security(("session" = [])),
    params(("reference" = String, Path, description = "Consent token or device user code")),
    request_body = ApproveRequest,
    responses(
        (status = 200, body = Decision),
        (status = 400, body = Problem, content_type = "application/problem+json"),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 404, body = Problem, content_type = "application/problem+json"),
        (status = 409, body = Problem, content_type = "application/problem+json"),
    )
)]
async fn approve(
    State(state): State<AppState>,
    session: WebSession,
    Path(reference): Path<String>,
    Json(request): Json<ApproveRequest>,
) -> Result<Json<Decision>, AuthError> {
    let asserted = ceremony::finish_authentication(
        &state,
        &Secret::from_text(&request.challenge),
        &request.credential,
        Assertion::Approval,
    )
    .await?;
    if asserted != session.user_id() {
        return Err(AuthError::InvalidCredentials);
    }
    let folders = match &request.scopes {
        Some(scopes) => {
            connections::scopes::decode(scopes).map_err(|_| AuthError::InvalidInput("scopes"))?
        }
        None => Folders::All,
    };
    let can_write = request.can_write.unwrap_or(true);
    let mut tx = state.pool().begin().await?;
    let (pending, client, audience) = lookup(&mut tx, &reference).await?;
    let name = match request.name {
        Some(name) => name,
        None => connections::proposed_name(&mut tx, session.user_id(), &client.name).await?,
    };
    let connection_id = connections::create(
        &mut tx,
        session.user_id(),
        &name,
        &folders,
        can_write,
        audience,
    )
    .await?;
    let decision = match pending {
        Pending::Authorization(row) => {
            consents::take(&mut tx, &consent_hash(&reference))
                .await?
                .ok_or(AuthError::NotFound)?;
            let code = Secret::generate();
            codes::insert(
                &mut tx,
                &code.hash(),
                client.id,
                connection_id,
                &row.redirect_uri,
                &row.code_challenge,
                CODE_SECONDS,
            )
            .await?;
            let target = redirect_target(
                &state,
                &row.redirect_uri,
                row.client_state.as_deref(),
                &[("code", code.expose())],
            );
            Decision {
                redirect_uri: Some(target),
            }
        }
        Pending::Device(user_code) => {
            if !devices::approve(&mut tx, user_code.as_str(), connection_id).await? {
                return Err(AuthError::NotFound);
            }
            Decision { redirect_uri: None }
        }
    };
    tx.commit().await?;
    Ok(Json(decision))
}

#[utoipa::path(
    post,
    path = "/api/oauth/consents/{reference}/deny",
    tag = "oauth",
    security(("session" = [])),
    params(("reference" = String, Path, description = "Consent token or device user code")),
    responses(
        (status = 200, body = Decision),
        (status = 401, body = Problem, content_type = "application/problem+json"),
        (status = 404, body = Problem, content_type = "application/problem+json"),
    )
)]
async fn deny(
    State(state): State<AppState>,
    _session: WebSession,
    Path(reference): Path<String>,
) -> Result<Json<Decision>, AuthError> {
    let mut tx = state.pool().begin().await?;
    let (pending, _client, _audience) = lookup(&mut tx, &reference).await?;
    let decision = match pending {
        Pending::Authorization(row) => {
            consents::take(&mut tx, &consent_hash(&reference))
                .await?
                .ok_or(AuthError::NotFound)?;
            Decision {
                redirect_uri: Some(redirect_target(
                    &state,
                    &row.redirect_uri,
                    row.client_state.as_deref(),
                    &[("error", OAuthErrorCode::AccessDenied.as_str())],
                )),
            }
        }
        Pending::Device(user_code) => {
            devices::deny(&mut tx, user_code.as_str()).await?;
            Decision { redirect_uri: None }
        }
    };
    tx.commit().await?;
    Ok(Json(decision))
}

fn consent_hash(reference: &str) -> crate::auth::token::SecretHash {
    Secret::from_text(reference).hash()
}

async fn lookup(
    conn: &mut PgConnection,
    reference: &str,
) -> Result<(Pending, Client, Audience), AuthError> {
    if let Some(user_code) = UserCode::parse(reference) {
        let device = devices::find_pending(conn, user_code.as_str())
            .await?
            .ok_or(AuthError::NotFound)?;
        let client = clients::by_id(conn, device.oauth_client_id)
            .await
            .map_err(|_| AuthError::NotFound)?;
        return Ok((Pending::Device(user_code), client, device.audience));
    }
    let row = consents::find(conn, &consent_hash(reference))
        .await?
        .ok_or(AuthError::NotFound)?;
    let client = clients::by_id(conn, row.oauth_client_id)
        .await
        .map_err(|_| AuthError::NotFound)?;
    let audience = row.audience;
    Ok((Pending::Authorization(row), client, audience))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_matches_the_registrable_domain_it_is_named_after() {
        assert!(name_matches_host("Claude", "claude.ai"));
        assert!(name_matches_host("Cursor", "www.cursor.com"));
        assert!(name_matches_host("ChatGPT Connector", "chatgpt.com"));
        assert!(name_matches_host("Acme Memory", "app.acme.example"));
        assert!(name_matches_host("Acme", "acme.co.uk"));
    }

    #[test]
    fn a_name_that_borrows_another_brand_is_flagged() {
        assert!(!name_matches_host("Claude", "evil.example"));
        assert!(!name_matches_host("Claude", "claude.attacker.io"));
        assert!(!name_matches_host("Claude", "claude-login.attacker.io"));
        assert!(!name_matches_host("Claude", "ai.example"));
    }

    #[test]
    fn a_name_without_letters_matches_nothing() {
        assert!(!name_matches_host("\u{1F642}", "claude.ai"));
    }
}
