use sqlx::PgConnection;

use crate::access::ConnectionId;
use crate::auth::token::SecretHash;
use crate::oauth::ids::OauthClientId;

#[derive(Debug)]
pub(crate) struct RefreshRow {
    pub(crate) id: i64,
    pub(crate) connection_id: ConnectionId,
    pub(crate) oauth_client_id: OauthClientId,
    pub(crate) is_rotated: bool,
    pub(crate) is_expired: bool,
}

#[derive(Debug)]
pub(crate) struct TokenOwner {
    pub(crate) connection_id: ConnectionId,
    pub(crate) oauth_client_id: OauthClientId,
}

pub(crate) async fn insert(
    conn: &mut PgConnection,
    connection_id: ConnectionId,
    oauth_client_id: OauthClientId,
    access: &SecretHash,
    refresh: &SecretHash,
    access_seconds: f64,
    refresh_seconds: f64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO oauth_tokens
          (connection_id, oauth_client_id, access_token_hash, refresh_token_hash,
           access_expires_at, refresh_expires_at)
        VALUES ($1, $2, $3, $4,
                now() + make_interval(secs => $5),
                now() + make_interval(secs => $6))
        "#,
        connection_id as ConnectionId,
        oauth_client_id as OauthClientId,
        access.as_bytes(),
        refresh.as_bytes(),
        access_seconds,
        refresh_seconds,
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub(crate) async fn lock_refresh(
    conn: &mut PgConnection,
    refresh: &SecretHash,
) -> Result<Option<RefreshRow>, sqlx::Error> {
    sqlx::query_as!(
        RefreshRow,
        r#"
        SELECT
          id,
          connection_id AS "connection_id: ConnectionId",
          oauth_client_id AS "oauth_client_id: OauthClientId",
          rotated_at IS NOT NULL AS "is_rotated!",
          refresh_expires_at <= now() AS "is_expired!"
        FROM oauth_tokens
        WHERE refresh_token_hash = $1
        FOR UPDATE
        "#,
        refresh.as_bytes(),
    )
    .fetch_optional(conn)
    .await
}

pub(crate) async fn mark_rotated(conn: &mut PgConnection, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE oauth_tokens SET rotated_at = now() WHERE id = $1",
        id
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub(crate) async fn connection_of_access(
    conn: &mut PgConnection,
    access: &SecretHash,
) -> Result<Option<ConnectionId>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT connection_id AS "connection_id: ConnectionId"
        FROM oauth_tokens
        WHERE access_token_hash = $1
          AND access_expires_at > now()
        "#,
        access.as_bytes(),
    )
    .fetch_optional(conn)
    .await
}

pub(crate) async fn owner_of_token(
    conn: &mut PgConnection,
    token: &SecretHash,
) -> Result<Option<TokenOwner>, sqlx::Error> {
    sqlx::query_as!(
        TokenOwner,
        r#"
        SELECT
          connection_id AS "connection_id: ConnectionId",
          oauth_client_id AS "oauth_client_id: OauthClientId"
        FROM oauth_tokens
        WHERE access_token_hash = $1
           OR refresh_token_hash = $1
        "#,
        token.as_bytes(),
    )
    .fetch_optional(conn)
    .await
}
