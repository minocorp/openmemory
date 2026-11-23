use std::fmt;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

const TOKEN_BYTES: usize = 32;

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct Secret(String);

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct SecretHash([u8; 32]);

impl Secret {
    pub(crate) fn generate() -> Self {
        let mut bytes = [0_u8; TOKEN_BYTES];
        rand::fill(&mut bytes);
        Self(URL_SAFE_NO_PAD.encode(bytes))
    }

    pub(crate) fn from_text(text: &str) -> Self {
        Self(text.to_owned())
    }

    pub(crate) fn expose(&self) -> &str {
        &self.0
    }

    pub(crate) fn hash(&self) -> SecretHash {
        SecretHash::of(self.expose())
    }
}

impl SecretHash {
    pub(crate) fn of(text: &str) -> Self {
        Self(Sha256::digest(text.as_bytes()).into())
    }

    pub(crate) fn from_slice(bytes: &[u8]) -> Option<Self> {
        <[u8; 32]>::try_from(bytes).ok().map(Self)
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub(crate) fn matches(&self, other: &Self) -> bool {
        self.0.ct_eq(&other.0).into()
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[redacted]")
    }
}

impl fmt::Debug for SecretHash {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[redacted]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_secrets_are_unique_and_url_safe() {
        let first = Secret::generate();
        let second = Secret::generate();
        assert_ne!(first, second);
        assert_eq!(first.expose().len(), 43);
        assert!(
            first
                .expose()
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
        );
    }

    #[test]
    fn hashes_compare_by_value() {
        let secret = Secret::generate();
        assert!(secret.hash().matches(&SecretHash::of(secret.expose())));
        assert!(!secret.hash().matches(&Secret::generate().hash()));
    }
}
