use std::sync::Arc;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use webauthn_rs::prelude::{RequestChallengeResponse, Webauthn, WebauthnBuilder, WebauthnResult};

use crate::config::PublicUrl;

const RELYING_PARTY_NAME: &str = "OpenMemory";
const CHALLENGE_BYTES: usize = 32;
const TIMEOUT_MILLISECONDS: u32 = 300_000;
const TRANSPORT_SETS: [&[&str]; 4] = [
    &["internal"],
    &["internal", "hybrid"],
    &["hybrid", "internal"],
    &["usb", "nfc"],
];

#[derive(Debug, Clone)]
pub(crate) struct Passkeys {
    webauthn: Arc<Webauthn>,
    rp_id: Arc<str>,
    decoy_key: [u8; 32],
}

impl Passkeys {
    pub(crate) fn new(public_url: &PublicUrl) -> WebauthnResult<Self> {
        let webauthn = WebauthnBuilder::new(public_url.rp_id(), public_url.origin())?
            .rp_name(RELYING_PARTY_NAME)
            .build()?;
        let mut decoy_key = [0_u8; 32];
        rand::fill(&mut decoy_key);
        Ok(Self {
            webauthn: Arc::new(webauthn),
            rp_id: Arc::from(public_url.rp_id()),
            decoy_key,
        })
    }

    pub(super) fn webauthn(&self) -> &Webauthn {
        &self.webauthn
    }

    // Mirrors what webauthn-rs 0.6.2 emits for a real passkey, with ids and transports stable per name so repeated probes agree.
    pub(super) fn decoy_challenge(
        &self,
        name: &str,
    ) -> serde_json::Result<RequestChallengeResponse> {
        let seed = Sha256::new()
            .chain_update(self.decoy_key)
            .chain_update(name.as_bytes())
            .finalize();
        let count = 1 + usize::from(seed[0] % 2);
        let allow_credentials: Vec<Value> = (0..count)
            .map(|index| {
                let id = Sha256::new()
                    .chain_update(seed)
                    .chain_update([u8::try_from(index).unwrap_or_default()])
                    .finalize();
                let transports = TRANSPORT_SETS[usize::from(id[0]) % TRANSPORT_SETS.len()];
                json!({
                    "type": "public-key",
                    "id": URL_SAFE_NO_PAD.encode(id),
                    "transports": transports,
                })
            })
            .collect();
        let mut challenge = [0_u8; CHALLENGE_BYTES];
        rand::fill(&mut challenge);
        serde_json::from_value(json!({
            "publicKey": {
                "challenge": URL_SAFE_NO_PAD.encode(challenge),
                "timeout": TIMEOUT_MILLISECONDS,
                "rpId": &*self.rp_id,
                "allowCredentials": allow_credentials,
                "userVerification": "required",
            }
        }))
    }
}
