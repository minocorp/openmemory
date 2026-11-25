use axum::extract::State;
use axum::routing::post;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use utoipa::{OpenApi, ToSchema};
use webauthn_rs::prelude::{PublicKeyCredential, RequestChallengeResponse};

use crate::auth::ceremony::{self, Assertion};
use crate::auth::token::Secret;
use crate::auth::{AuthError, WebSession};
use crate::state::AppState;
use openmemory_api::Problem;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Action {
    AddPasskey,
    RemovePasskey,
    CreateInvite,
    SetAdmin,
    Export,
    CreateToken,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct Confirmation {
    challenge: String,
    #[schema(value_type = Object)]
    credential: PublicKeyCredential,
}

#[derive(Debug, Deserialize, ToSchema)]
pub(crate) struct Confirmed {
    pub(crate) confirmation: Confirmation,
}

#[derive(Debug, Deserialize, ToSchema)]
struct ConfirmationStart {
    action: Action,
}

#[derive(Debug, Serialize, ToSchema)]
struct ConfirmationChallenge {
    challenge: String,
    #[schema(value_type = Object)]
    options: RequestChallengeResponse,
}

#[derive(OpenApi)]
#[openapi(
    paths(start_confirmation),
    components(schemas(
        Action,
        Confirmation,
        Confirmed,
        ConfirmationStart,
        ConfirmationChallenge
    ))
)]
pub(super) struct Doc;

pub(super) fn router() -> Router<AppState> {
    Router::new().route("/api/auth/confirm", post(start_confirmation))
}

impl Confirmation {
    pub(crate) async fn verify(
        &self,
        state: &AppState,
        session: &WebSession,
        action: Action,
    ) -> Result<(), AuthError> {
        let asserted = ceremony::finish_authentication(
            state,
            &Secret::from_text(&self.challenge),
            &self.credential,
            Assertion::Confirm(action),
        )
        .await?;
        if asserted == session.user_id() {
            Ok(())
        } else {
            Err(AuthError::InvalidCredentials)
        }
    }
}

#[utoipa::path(
    post,
    path = "/api/auth/confirm",
    tag = "auth",
    security(("session" = [])),
    request_body = ConfirmationStart,
    responses((status = 200, body = ConfirmationChallenge), (status = 401, body = Problem, content_type = "application/problem+json"))
)]
async fn start_confirmation(
    State(state): State<AppState>,
    session: WebSession,
    Json(request): Json<ConfirmationStart>,
) -> Result<Json<ConfirmationChallenge>, AuthError> {
    let (challenge, options) = ceremony::start_authentication(
        &state,
        session.user_id(),
        Assertion::Confirm(request.action),
    )
    .await?;
    Ok(Json(ConfirmationChallenge {
        challenge: challenge.expose().to_owned(),
        options,
    }))
}
