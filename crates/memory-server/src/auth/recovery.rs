use rand::RngExt;
use sqlx::PgConnection;

use crate::access::UserId;
use crate::auth::AuthError;
use crate::auth::queries::recovery;
use crate::auth::token::SecretHash;

const GROUPS: usize = 8;
const GROUP_LENGTH: usize = 4;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RecoveryCode(String);

impl RecoveryCode {
    pub(crate) fn generate() -> Self {
        let digits: String = (0..GROUPS * GROUP_LENGTH)
            .map(|_| char::from_digit(rand::rng().random_range(0..16), 16).unwrap_or('0'))
            .collect();
        Self(digits)
    }

    pub(crate) fn parse(input: &str) -> Self {
        Self(
            input
                .chars()
                .filter(char::is_ascii_alphanumeric)
                .map(|c| c.to_ascii_lowercase())
                .collect(),
        )
    }

    pub(crate) fn hash(&self) -> SecretHash {
        SecretHash::of(&self.0)
    }

    pub(crate) fn display(&self) -> String {
        self.0
            .as_bytes()
            .chunks(GROUP_LENGTH)
            .filter_map(|chunk| std::str::from_utf8(chunk).ok())
            .collect::<Vec<_>>()
            .join("-")
    }
}

pub(crate) async fn issue(
    conn: &mut PgConnection,
    user_id: UserId,
) -> Result<RecoveryCode, AuthError> {
    let code = RecoveryCode::generate();
    recovery::insert(conn, user_id, &code.hash()).await?;
    Ok(code)
}

pub(crate) async fn is_valid(
    conn: &mut PgConnection,
    user_id: UserId,
    code: &RecoveryCode,
) -> Result<bool, AuthError> {
    Ok(recovery::is_unused(conn, user_id, &code.hash()).await?)
}

pub(crate) async fn consume(
    conn: &mut PgConnection,
    user_id: UserId,
    code_hash: &SecretHash,
) -> Result<bool, AuthError> {
    Ok(recovery::consume(conn, user_id, code_hash).await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displayed_codes_parse_back_to_the_same_hash() {
        let code = RecoveryCode::generate();
        let shown = code.display();
        assert_eq!(shown.len(), GROUPS * GROUP_LENGTH + GROUPS - 1);
        assert!(
            RecoveryCode::parse(&shown.to_uppercase())
                .hash()
                .matches(&code.hash())
        );
    }

    #[test]
    fn generated_codes_differ() {
        assert_ne!(RecoveryCode::generate(), RecoveryCode::generate());
    }
}
