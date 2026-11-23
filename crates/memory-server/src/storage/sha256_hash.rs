use std::fmt::Write;

use sha2::{Digest, Sha256};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct Sha256Hash([u8; 32]);

impl Sha256Hash {
    pub(crate) fn of(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }

    pub(crate) fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    pub(crate) fn to_hex(self) -> String {
        self.0
            .iter()
            .fold(String::with_capacity(64), |mut hex, byte| {
                let _ = write!(hex, "{byte:02x}");
                hex
            })
    }
}

impl TryFrom<Vec<u8>> for Sha256Hash {
    type Error = Vec<u8>;

    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        <[u8; 32]>::try_from(bytes).map(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_matches_the_known_digest_of_empty_input() {
        assert_eq!(
            Sha256Hash::of(b"").to_hex(),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
