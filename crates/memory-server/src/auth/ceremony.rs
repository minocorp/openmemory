use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::{Value, json};
use sqlx::PgConnection;
use webauthn_rs::prelude::{
    AuthenticationResult, CreationChallengeResponse, CredentialID, DiscoverableAuthentication,
    Passkey, PasskeyAuthentication, PasskeyRegistration, PublicKeyCredential,
    RegisterPublicKeyCredential, RequestChallengeResponse, Uuid,
};

use crate::access::UserId;
use crate::auth::AuthError;
use crate::auth::confirmation::Action;
use crate::auth::ids::InviteId;
use crate::auth::queries::challenges::{self, ChallengeKind};
use crate::auth::queries::passkeys as passkey_queries;
use crate::auth::queries::users;
use crate::auth::token::Secret;
use crate::state::AppState;

const CHALLENGE_SECONDS: f64 = 300.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) enum Purpose {
    Bootstrap {
        username: String,
        display_name: String,
        user_handle: Uuid,
    },
    Invite {
        invite_id: InviteId,
        username: String,
        display_name: String,
        user_handle: Uuid,
    },
    AddPasskey {
        user_id: UserId,
    },
    Recovery {
        user_id: UserId,
        code_hash: Vec<u8>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) enum Assertion {
    Login,
    Approval,
    Purge,
    Confirm(Action),
}

#[derive(Serialize, Deserialize)]
struct RegistrationState {
    registration: PasskeyRegistration,
    purpose: Purpose,
    passkey_name: String,
}

#[derive(Serialize, Deserialize)]
struct AuthenticationState {
    authentication: PasskeyAuthentication,
    user_id: UserId,
    assertion: Assertion,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DiscoverableState {
    authentication: DiscoverableAuthentication,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Account<'a> {
    pub(crate) user_handle: Uuid,
    pub(crate) username: &'a str,
    pub(crate) display_name: &'a str,
}

#[derive(Debug)]
pub(crate) struct FinishedRegistration {
    pub(crate) purpose: Purpose,
    pub(crate) passkey_name: String,
    pub(crate) passkey: Passkey,
}

impl Purpose {
    fn user_id(&self) -> Option<UserId> {
        match self {
            Self::Bootstrap { .. } | Self::Invite { .. } => None,
            Self::AddPasskey { user_id } | Self::Recovery { user_id, .. } => Some(*user_id),
        }
    }
}

pub(crate) fn user_handle(bytes: &[u8]) -> Result<Uuid, AuthError> {
    Uuid::from_slice(bytes).map_err(|_| AuthError::InvalidToken)
}

pub(crate) async fn account_of(
    conn: &mut PgConnection,
    user_id: UserId,
) -> Result<users::UserRow, AuthError> {
    users::find(conn, user_id)
        .await?
        .ok_or(AuthError::Unauthenticated)
}

// webauthn-rs 0.6.2 asks for non-discoverable credentials and does not export the option types; passkey autofill needs discoverable ones.
fn require_discoverable(
    options: CreationChallengeResponse,
) -> Result<CreationChallengeResponse, serde_json::Error> {
    let mut value = serde_json::to_value(&options)?;
    if let Some(selection) = value.pointer_mut("/publicKey/authenticatorSelection") {
        selection["residentKey"] = json!("required");
        selection["requireResidentKey"] = json!(true);
    }
    serde_json::from_value(value)
}

pub(crate) async fn start_registration(
    state: &AppState,
    account: Account<'_>,
    exclude: Vec<CredentialID>,
    purpose: Purpose,
    passkey_name: String,
) -> Result<(Secret, CreationChallengeResponse), AuthError> {
    let (options, registration) = state
        .passkeys()
        .webauthn()
        .start_passkey_registration(
            account.user_handle,
            account.username,
            account.display_name,
            Some(exclude),
        )
        .map_err(AuthError::Webauthn)?;
    let options = require_discoverable(options)?;
    let challenge = Secret::generate();
    let stored = RegistrationState {
        registration,
        purpose: purpose.clone(),
        passkey_name,
    };
    let mut conn = state.pool().acquire().await?;
    challenges::delete_expired(&mut conn).await?;
    challenges::insert(
        &mut conn,
        &challenge.hash(),
        ChallengeKind::Registration,
        purpose.user_id(),
        &serde_json::to_value(&stored)?,
        CHALLENGE_SECONDS,
    )
    .await?;
    Ok((challenge, options))
}

pub(crate) async fn finish_registration(
    state: &AppState,
    challenge: &Secret,
    credential: &RegisterPublicKeyCredential,
) -> Result<FinishedRegistration, AuthError> {
    let stored: RegistrationState =
        take_state(state, challenge, ChallengeKind::Registration).await?;
    let passkey = state
        .passkeys()
        .webauthn()
        .finish_passkey_registration(credential, &stored.registration)
        .map_err(AuthError::PasskeyRejected)?;
    Ok(FinishedRegistration {
        purpose: stored.purpose,
        passkey_name: stored.passkey_name,
        passkey,
    })
}

pub(crate) async fn start_authentication(
    state: &AppState,
    user_id: UserId,
    assertion: Assertion,
) -> Result<(Secret, RequestChallengeResponse), AuthError> {
    let mut conn = state.pool().acquire().await?;
    let passkeys = load_passkeys(&mut conn, user_id).await?;
    if passkeys.is_empty() {
        return Err(AuthError::InvalidCredentials);
    }
    let (options, authentication) = state
        .passkeys()
        .webauthn()
        .start_passkey_authentication(&passkeys)
        .map_err(AuthError::Webauthn)?;
    let challenge = Secret::generate();
    let stored = AuthenticationState {
        authentication,
        user_id,
        assertion,
    };
    challenges::insert(
        &mut conn,
        &challenge.hash(),
        ChallengeKind::Authentication,
        Some(user_id),
        &serde_json::to_value(&stored)?,
        CHALLENGE_SECONDS,
    )
    .await?;
    Ok((challenge, options))
}

pub(crate) async fn start_decoy_authentication(
    state: &AppState,
    name: &str,
) -> Result<(Secret, RequestChallengeResponse), AuthError> {
    let challenge = Secret::generate();
    let mut conn = state.pool().acquire().await?;
    challenges::insert(
        &mut conn,
        &challenge.hash(),
        ChallengeKind::Authentication,
        None,
        &json!({}),
        CHALLENGE_SECONDS,
    )
    .await?;
    Ok((challenge, state.passkeys().decoy_challenge(name)?))
}

pub(crate) async fn finish_authentication(
    state: &AppState,
    challenge: &Secret,
    credential: &PublicKeyCredential,
    expected: Assertion,
) -> Result<UserId, AuthError> {
    let stored: AuthenticationState = take_state(state, challenge, ChallengeKind::Authentication)
        .await
        // Decoy challenges store no state and must fail like a wrong assertion.
        .map_err(|error| match error {
            AuthError::State(_) => AuthError::InvalidCredentials,
            other => other,
        })?;
    if stored.assertion != expected {
        return Err(AuthError::InvalidToken);
    }
    let result = state
        .passkeys()
        .webauthn()
        .finish_passkey_authentication(credential, &stored.authentication)
        .map_err(|_| AuthError::InvalidCredentials)?;
    let mut tx = state.pool().begin().await?;
    record_use(&mut tx, stored.user_id, &result).await?;
    tx.commit().await?;
    Ok(stored.user_id)
}

pub(crate) async fn start_discoverable_authentication(
    state: &AppState,
) -> Result<(Secret, RequestChallengeResponse), AuthError> {
    let (options, authentication) = state
        .passkeys()
        .webauthn()
        .start_discoverable_authentication()
        .map_err(AuthError::Webauthn)?;
    let challenge = Secret::generate();
    let mut conn = state.pool().acquire().await?;
    challenges::insert(
        &mut conn,
        &challenge.hash(),
        ChallengeKind::Authentication,
        None,
        &serde_json::to_value(&DiscoverableState { authentication })?,
        CHALLENGE_SECONDS,
    )
    .await?;
    Ok((challenge, options))
}

pub(crate) async fn finish_discoverable_authentication(
    state: &AppState,
    challenge: &Secret,
    credential: &PublicKeyCredential,
) -> Result<UserId, AuthError> {
    let stored: DiscoverableState = take_state(state, challenge, ChallengeKind::Authentication)
        .await
        .map_err(|error| match error {
            AuthError::State(_) => AuthError::InvalidCredentials,
            other => other,
        })?;
    let webauthn = state.passkeys().webauthn();
    let (handle, credential_id) = webauthn
        .identify_discoverable_authentication(credential)
        .map_err(|_| AuthError::InvalidCredentials)?;
    let mut conn = state.pool().acquire().await?;
    let user = users::find_by_user_handle(&mut conn, handle.as_bytes())
        .await?
        .ok_or(AuthError::InvalidCredentials)?;
    let passkey = load_passkeys(&mut conn, user.id)
        .await?
        .into_iter()
        .find(|passkey| AsRef::<[u8]>::as_ref(passkey.cred_id()) == credential_id)
        .ok_or(AuthError::InvalidCredentials)?;
    let result = webauthn
        .finish_discoverable_authentication_v2(credential, stored.authentication, &passkey)
        .map_err(|_| AuthError::InvalidCredentials)?;
    record_use(&mut conn, user.id, &result).await?;
    Ok(user.id)
}

async fn record_use(
    conn: &mut PgConnection,
    user_id: UserId,
    result: &AuthenticationResult,
) -> Result<(), AuthError> {
    for mut passkey in load_passkeys(conn, user_id).await? {
        if passkey.update_credential(result).is_some() {
            passkey_queries::record_use(
                conn,
                passkey.cred_id().as_ref(),
                &serde_json::to_value(&passkey)?,
            )
            .await?;
        }
    }
    Ok(())
}

pub(crate) async fn load_passkeys(
    conn: &mut PgConnection,
    user_id: UserId,
) -> Result<Vec<Passkey>, AuthError> {
    passkey_queries::list(conn, user_id)
        .await?
        .into_iter()
        .map(|row| serde_json::from_value(row.credential).map_err(AuthError::from))
        .collect()
}

pub(crate) async fn store_passkey(
    conn: &mut PgConnection,
    user_id: UserId,
    passkey: &Passkey,
    name: &str,
) -> Result<(), AuthError> {
    let value: Value = serde_json::to_value(passkey)?;
    passkey_queries::insert(conn, user_id, passkey.cred_id().as_ref(), &value, name)
        .await
        .map_err(|error| {
            if crate::auth::violated_constraint(&error) == Some("passkeys_credential_id_key") {
                AuthError::PasskeyTaken
            } else {
                AuthError::Database(error)
            }
        })
}

async fn take_state<T: DeserializeOwned>(
    state: &AppState,
    challenge: &Secret,
    kind: ChallengeKind,
) -> Result<T, AuthError> {
    let mut conn = state.pool().acquire().await?;
    let row = challenges::take(&mut conn, &challenge.hash())
        .await?
        .filter(|row| row.is(kind))
        .ok_or(AuthError::InvalidToken)?;
    Ok(serde_json::from_value(row.state)?)
}
