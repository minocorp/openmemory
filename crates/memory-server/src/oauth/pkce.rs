use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

const CHALLENGE_LENGTH: usize = 43;
const VERIFIER_LENGTHS: std::ops::RangeInclusive<usize> = 43..=128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CodeChallenge(String);

impl CodeChallenge {
    pub(crate) fn parse(input: &str) -> Option<Self> {
        let valid = input.len() == CHALLENGE_LENGTH
            && input
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        valid.then(|| Self(input.to_owned()))
    }

    pub(crate) fn from_stored(value: String) -> Self {
        Self(value)
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn verifies(&self, verifier: &str) -> bool {
        let unreserved = |c: char| c.is_ascii_alphanumeric() || "-._~".contains(c);
        if !VERIFIER_LENGTHS.contains(&verifier.len()) || !verifier.chars().all(unreserved) {
            return false;
        }
        let computed = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
        computed.as_bytes().ct_eq(self.0.as_bytes()).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
    const CHALLENGE: &str = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";

    #[test]
    fn the_rfc_7636_appendix_b_vector_verifies() {
        let challenge = CodeChallenge::parse(CHALLENGE).unwrap();
        assert!(challenge.verifies(VERIFIER));
    }

    #[test]
    fn wrong_malformed_and_plain_verifiers_fail() {
        let challenge = CodeChallenge::parse(CHALLENGE).unwrap();
        assert!(!challenge.verifies("a".repeat(43).as_str()));
        assert!(!challenge.verifies(CHALLENGE));
        assert!(!challenge.verifies("short"));
        assert!(!challenge.verifies(&format!("{VERIFIER}!")));
    }

    #[test]
    fn challenges_must_be_43_url_safe_characters() {
        assert!(CodeChallenge::parse(CHALLENGE).is_some());
        assert!(CodeChallenge::parse("short").is_none());
        let bad: String = CHALLENGE.chars().skip(1).chain(['+']).collect();
        assert!(CodeChallenge::parse(&bad).is_none());
    }
}
