use sqlx::PgConnection;

use crate::access::{Audience, ConnectionId};
use crate::auth::token::SecretHash;
use crate::oauth::ids::OauthClientId;

#[derive(Debug)]
pub(crate) struct PendingDevice {
    pub(crate) oauth_client_id: OauthClientId,
    pub(crate) audience: Audience,
}

#[derive(Debug)]
pub(crate) struct DeviceRow {
    pub(crate) id: i64,
    pub(crate) oauth_client_id: OauthClientId,
    pub(crate) connection_id: Option<ConnectionId>,
    pub(crate) is_denied: bool,
    pub(crate) is_expired: bool,
    pub(crate) is_polling_too_fast: bool,
}

pub(crate) async fn insert(
    conn: &mut PgConnection,
    device_code: &SecretHash,
    user_code: &str,
    oauth_client_id: OauthClientId,
    audience: Audience,
    lifetime_seconds: f64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO device_codes (device_code_hash, user_code, oauth_client_id, audience, expires_at)
        VALUES ($1, $2, $3, $4, now() + make_interval(secs => $5))
        "#,
        device_code.as_bytes(),
        user_code,
        oauth_client_id as OauthClientId,
        audience as Audience,
        lifetime_seconds,
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub(crate) async fn find_pending(
    conn: &mut PgConnection,
    user_code: &str,
) -> Result<Option<PendingDevice>, sqlx::Error> {
    sqlx::query_as!(
        PendingDevice,
        r#"
        SELECT
          oauth_client_id AS "oauth_client_id: OauthClientId",
          audience AS "audience: Audience"
        FROM device_codes
        WHERE user_code = $1
          AND connection_id IS NULL
          AND denied_at IS NULL
          AND expires_at > now()
        "#,
        user_code,
    )
    .fetch_optional(conn)
    .await
}

pub(crate) async fn approve(
    conn: &mut PgConnection,
    user_code: &str,
    connection_id: ConnectionId,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        UPDATE device_codes
        SET connection_id = $2
        WHERE user_code = $1
          AND connection_id IS NULL
          AND denied_at IS NULL
          AND expires_at > now()
        "#,
        user_code,
        connection_id as ConnectionId,
    )
    .execute(conn)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub(crate) async fn deny(conn: &mut PgConnection, user_code: &str) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        UPDATE device_codes
        SET denied_at = now()
        WHERE user_code = $1
          AND connection_id IS NULL
          AND denied_at IS NULL
          AND expires_at > now()
        "#,
        user_code,
    )
    .execute(conn)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub(crate) async fn lock_by_device_code(
    conn: &mut PgConnection,
    device_code: &SecretHash,
    interval_seconds: f64,
) -> Result<Option<DeviceRow>, sqlx::Error> {
    sqlx::query_as!(
        DeviceRow,
        r#"
        SELECT
          id,
          oauth_client_id AS "oauth_client_id: OauthClientId",
          connection_id AS "connection_id: ConnectionId",
          denied_at IS NOT NULL AS "is_denied!",
          expires_at <= now() AS "is_expired!",
          COALESCE(last_polled_at > now() - make_interval(secs => $2), false) AS "is_polling_too_fast!"
        FROM device_codes
        WHERE device_code_hash = $1
        FOR UPDATE
        "#,
        device_code.as_bytes(),
        interval_seconds,
    )
    .fetch_optional(conn)
    .await
}

pub(crate) async fn record_poll(conn: &mut PgConnection, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE device_codes SET last_polled_at = now() WHERE id = $1",
        id
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub(crate) async fn delete_expired(conn: &mut PgConnection) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM device_codes WHERE expires_at <= now()")
        .execute(conn)
        .await?;
    Ok(())
}

pub(crate) async fn delete(conn: &mut PgConnection, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM device_codes WHERE id = $1", id)
        .execute(conn)
        .await?;
    Ok(())
}
