use sqlx::PgConnection;

use crate::access::ConnectionId;
use crate::auth::token::SecretHash;
use crate::oauth::ids::OauthClientId;

#[derive(Debug)]
pub(crate) struct CodeRow {
    pub(crate) oauth_client_id: OauthClientId,
    pub(crate) connection_id: ConnectionId,
    pub(crate) redirect_uri: String,
    pub(crate) code_challenge: String,
    pub(crate) is_expired: bool,
}

pub(crate) async fn insert(
    conn: &mut PgConnection,
    code: &SecretHash,
    oauth_client_id: OauthClientId,
    connection_id: ConnectionId,
    redirect_uri: &str,
    code_challenge: &str,
    lifetime_seconds: f64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO oauth_codes
          (code_hash, oauth_client_id, connection_id, redirect_uri, code_challenge, expires_at)
        VALUES ($1, $2, $3, $4, $5, now() + make_interval(secs => $6))
        "#,
        code.as_bytes(),
        oauth_client_id as OauthClientId,
        connection_id as ConnectionId,
        redirect_uri,
        code_challenge,
        lifetime_seconds,
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub(crate) async fn burn(
    conn: &mut PgConnection,
    code: &SecretHash,
) -> Result<Option<CodeRow>, sqlx::Error> {
    sqlx::query_as!(
        CodeRow,
        r#"
        UPDATE oauth_codes
        SET used_at = now()
        WHERE code_hash = $1
          AND used_at IS NULL
        RETURNING
          oauth_client_id AS "oauth_client_id: OauthClientId",
          connection_id AS "connection_id: ConnectionId",
          redirect_uri,
          code_challenge,
          expires_at <= now() AS "is_expired!"
        "#,
        code.as_bytes(),
    )
    .fetch_optional(conn)
    .await
}

pub(crate) async fn connection_of_used(
    conn: &mut PgConnection,
    code: &SecretHash,
) -> Result<Option<ConnectionId>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT connection_id AS "connection_id: ConnectionId"
        FROM oauth_codes
        WHERE code_hash = $1
          AND used_at IS NOT NULL
        "#,
        code.as_bytes(),
    )
    .fetch_optional(conn)
    .await
}
