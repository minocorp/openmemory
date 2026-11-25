use object_store::path::Path;

use crate::storage::Sha256Hash;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BlobKey(String);

impl BlobKey {
    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    pub(crate) fn to_path(&self) -> Path {
        Path::from(self.0.as_str())
    }
}

impl From<&Sha256Hash> for BlobKey {
    fn from(hash: &Sha256Hash) -> Self {
        let hex = hash.to_hex();
        let (shard, rest) = hex.split_at(2);
        Self(format!("{shard}/{rest}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_are_sharded_by_the_first_hash_byte() {
        let key = BlobKey::from(&Sha256Hash::of(b""));
        assert_eq!(
            key.as_str(),
            "e3/b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
