use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};

const VERIFIER_BYTES: usize = 32;

pub(crate) struct Pkce {
    pub(crate) verifier: String,
    pub(crate) challenge: String,
}

impl Pkce {
    pub(crate) fn generate() -> Self {
        Self::from_verifier(random_text())
    }

    fn from_verifier(verifier: String) -> Self {
        let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        Self {
            verifier,
            challenge,
        }
    }
}

pub(crate) fn random_text() -> String {
    let mut bytes = [0_u8; VERIFIER_BYTES];
    rand::fill(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_challenge_matches_the_rfc_7636_example() {
        let pkce = Pkce::from_verifier("dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk".to_owned());
        assert_eq!(
            pkce.challenge,
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM"
        );
    }

    #[test]
    fn generated_values_are_unique_and_url_safe() {
        let (first, second) = (Pkce::generate(), Pkce::generate());
        assert_ne!(first.verifier, second.verifier);
        assert_eq!(first.verifier.len(), 43);
        assert!(
            first
                .verifier
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
    }
}
