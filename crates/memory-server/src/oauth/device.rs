use axum::extract::State;
use axum::{Form, Json};
use rand::RngExt;
use serde::Deserialize;
use serde_json::{Value, json};

use crate::auth::token::Secret;
use crate::oauth::clients;
use crate::oauth::error::{OAuthError, OAuthErrorCode};
use crate::oauth::queries::devices;
use crate::state::AppState;

pub(super) const DEVICE_SECONDS: f64 = 600.0;
pub(super) const POLL_INTERVAL_SECONDS: u32 = 5;
const ALPHABET: &[u8] = b"BCDFGHJKLMNPQRSTVWXZ";
const HALF_LENGTH: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UserCode(String);

#[derive(Debug, Deserialize)]
pub(super) struct DeviceRequest {
    client_id: Option<String>,
    resource: Option<String>,
}

impl UserCode {
    pub(crate) fn generate() -> Self {
        let letters: Vec<char> = (0..HALF_LENGTH * 2)
            .map(|_| char::from(ALPHABET[rand::rng().random_range(0..ALPHABET.len())]))
            .collect();
        let (first, second) = letters.split_at(HALF_LENGTH);
        Self(format!(
            "{}-{}",
            first.iter().collect::<String>(),
            second.iter().collect::<String>()
        ))
    }

    pub(crate) fn parse(input: &str) -> Option<Self> {
        let letters: String = input
            .chars()
            .filter(|c| !matches!(c, '-' | ' '))
            .map(|c| c.to_ascii_uppercase())
            .collect();
        let valid = letters.len() == HALF_LENGTH * 2
            && letters.bytes().all(|byte| ALPHABET.contains(&byte));
        valid.then(|| {
            let (first, second) = letters.split_at(HALF_LENGTH);
            Self(format!("{first}-{second}"))
        })
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

pub(super) async fn start(
    State(state): State<AppState>,
    Form(request): Form<DeviceRequest>,
) -> Result<Json<Value>, OAuthError> {
    let client_id = request
        .client_id
        .as_deref()
        .ok_or(OAuthErrorCode::InvalidRequest)?;
    let client = clients::resolve(&state, client_id).await?;
    let audience = request
        .resource
        .as_deref()
        .and_then(|resource| state.public_url().audience_of(resource))
        .ok_or(OAuthErrorCode::InvalidTarget)?;
    let device_code = Secret::generate();
    let user_code = UserCode::generate();
    let mut conn = state.pool().acquire().await?;
    devices::insert(
        &mut conn,
        &device_code.hash(),
        user_code.as_str(),
        client.id,
        audience,
        DEVICE_SECONDS,
    )
    .await?;
    let verification_uri = state.public_url().join("/device");
    Ok(Json(json!({
        "device_code": device_code.expose(),
        "user_code": user_code.as_str(),
        "verification_uri": verification_uri,
        "verification_uri_complete": format!("{verification_uri}?user_code={}", user_code.as_str()),
        "expires_in": 600,
        "interval": POLL_INTERVAL_SECONDS,
    })))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_codes_round_trip_through_typing() {
        let code = UserCode::generate();
        assert_eq!(code.as_str().len(), 9);
        let typed = code.as_str().replace('-', "").to_lowercase();
        assert_eq!(UserCode::parse(&typed), Some(code));
    }

    #[test]
    fn lookalike_and_wrong_length_codes_are_rejected() {
        assert!(UserCode::parse("ABCD-EFGH").is_none());
        assert!(UserCode::parse("BCDF-GHJ").is_none());
        assert!(UserCode::parse("BCDF-GHJK-L").is_none());
        assert!(UserCode::parse("BCDF-GHJK").is_some());
    }
}
