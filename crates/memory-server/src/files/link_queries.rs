use sqlx::PgConnection;
use time::OffsetDateTime;

use crate::access::{ConnectionId, UserId};
use crate::storage::Sha256Hash;

pub(super) struct ClaimedLink {
    pub(super) id: i64,
    pub(super) path: String,
    pub(super) connection_id: ConnectionId,
}

pub(super) struct ConnectionRow {
    pub(super) user_id: UserId,
    pub(super) user_name: String,
    pub(super) name: String,
    pub(super) scopes: Vec<String>,
    pub(super) can_write: bool,
}

pub(super) async fn insert_link(
    conn: &mut PgConnection,
    token_hash: &Sha256Hash,
    path: &str,
    connection_id: ConnectionId,
    lifetime_minutes: i32,
) -> Result<OffsetDateTime, sqlx::Error> {
    sqlx::query_scalar!(
        r#"INSERT INTO upload_links (token_hash, path, connection_id, expires_at)
VALUES ($1, $2, $3, now() + make_interval(mins => $4))
RETURNING expires_at"#,
        token_hash.as_bytes(),
        path,
        connection_id.0,
        lifetime_minutes,
    )
    .fetch_one(conn)
    .await
}

pub(super) async fn claim_link(
    conn: &mut PgConnection,
    token_hash: &Sha256Hash,
) -> Result<Option<ClaimedLink>, sqlx::Error> {
    sqlx::query_as!(
        ClaimedLink,
        r#"UPDATE upload_links
SET used_at = now()
WHERE token_hash = $1
  AND used_at IS NULL
  AND expires_at > now()
RETURNING id, path, connection_id AS "connection_id: ConnectionId""#,
        token_hash.as_bytes(),
    )
    .fetch_optional(conn)
    .await
}

pub(super) struct OpenLink {
    pub(super) path: String,
    pub(super) expires_at: OffsetDateTime,
}

pub(super) async fn find_open_link(
    conn: &mut PgConnection,
    token_hash: &Sha256Hash,
) -> Result<Option<OpenLink>, sqlx::Error> {
    sqlx::query_as!(
        OpenLink,
        r#"SELECT path, expires_at
FROM upload_links
WHERE token_hash = $1
  AND used_at IS NULL
  AND expires_at > now()"#,
        token_hash.as_bytes(),
    )
    .fetch_optional(conn)
    .await
}

pub(super) async fn release_link(conn: &mut PgConnection, id: i64) -> Result<(), sqlx::Error> {
    sqlx::query!("UPDATE upload_links SET used_at = NULL WHERE id = $1", id)
        .execute(conn)
        .await?;
    Ok(())
}

pub(super) async fn find_live_connection(
    conn: &mut PgConnection,
    id: ConnectionId,
) -> Result<Option<ConnectionRow>, sqlx::Error> {
    sqlx::query_as!(
        ConnectionRow,
        r#"SELECT
  c.user_id AS "user_id: UserId",
  u.username AS user_name,
  c.name,
  c.scopes,
  c.can_write
FROM connections AS c
JOIN users AS u ON u.id = c.user_id
WHERE c.id = $1
  AND c.revoked_at IS NULL"#,
        id.0,
    )
    .fetch_optional(conn)
    .await
}
