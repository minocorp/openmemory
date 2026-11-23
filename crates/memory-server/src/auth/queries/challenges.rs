use serde_json::Value;
use sqlx::PgConnection;

use crate::access::UserId;
use crate::auth::token::SecretHash;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChallengeKind {
    Registration,
    Authentication,
}

impl ChallengeKind {
    fn as_text(self) -> &'static str {
        match self {
            Self::Registration => "registration",
            Self::Authentication => "authentication",
        }
    }
}

#[derive(Debug)]
pub(crate) struct ChallengeRow {
    pub(crate) kind: String,
    pub(crate) state: Value,
}

impl ChallengeRow {
    pub(crate) fn is(&self, kind: ChallengeKind) -> bool {
        self.kind == kind.as_text()
    }
}

pub(crate) async fn insert(
    conn: &mut PgConnection,
    token: &SecretHash,
    kind: ChallengeKind,
    user_id: Option<UserId>,
    state: &Value,
    lifetime_seconds: f64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO webauthn_challenges (token_hash, kind, user_id, state, expires_at)
        VALUES ($1, $2, $3, $4, now() + make_interval(secs => $5))
        "#,
        token.as_bytes(),
        kind.as_text(),
        user_id as Option<UserId>,
        state,
        lifetime_seconds,
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub(crate) async fn take(
    conn: &mut PgConnection,
    token: &SecretHash,
) -> Result<Option<ChallengeRow>, sqlx::Error> {
    sqlx::query_as!(
        ChallengeRow,
        r#"
        DELETE FROM webauthn_challenges
        WHERE token_hash = $1
          AND expires_at > now()
        RETURNING kind, state
        "#,
        token.as_bytes(),
    )
    .fetch_optional(conn)
    .await
}

pub(crate) async fn delete_expired(conn: &mut PgConnection) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM webauthn_challenges WHERE expires_at <= now()")
        .execute(conn)
        .await?;
    Ok(())
}
