use sqlx::PgConnection;
use time::OffsetDateTime;

use crate::access::UserId;
use crate::auth::ids::InviteId;
use crate::auth::token::SecretHash;

#[derive(Debug)]
pub(crate) struct InviteRow {
    pub(crate) id: InviteId,
    pub(crate) inviter: String,
    pub(crate) inviter_display_name: String,
    pub(crate) created_at: OffsetDateTime,
    pub(crate) expires_at: OffsetDateTime,
}

pub(crate) async fn insert(
    conn: &mut PgConnection,
    inviter_id: UserId,
    token: &SecretHash,
    lifetime_seconds: f64,
) -> Result<InviteId, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        INSERT INTO invites (inviter_id, token_hash, expires_at)
        VALUES ($1, $2, now() + make_interval(secs => $3))
        RETURNING id AS "id: InviteId"
        "#,
        inviter_id as UserId,
        token.as_bytes(),
        lifetime_seconds,
    )
    .fetch_one(conn)
    .await
}

pub(crate) async fn list_live(conn: &mut PgConnection) -> Result<Vec<InviteRow>, sqlx::Error> {
    sqlx::query_as!(
        InviteRow,
        r#"
        SELECT
          i.id AS "id: InviteId",
          u.username AS inviter,
          u.display_name AS inviter_display_name,
          i.created_at,
          i.expires_at
        FROM invites AS i
        JOIN users AS u ON u.id = i.inviter_id
        WHERE i.used_at IS NULL
          AND i.expires_at > now()
        ORDER BY i.created_at DESC, i.id DESC
        "#,
    )
    .fetch_all(conn)
    .await
}

pub(crate) async fn find_live(
    conn: &mut PgConnection,
    token: &SecretHash,
) -> Result<Option<InviteId>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT id AS "id: InviteId"
        FROM invites
        WHERE token_hash = $1
          AND used_at IS NULL
          AND expires_at > now()
        "#,
        token.as_bytes(),
    )
    .fetch_optional(conn)
    .await
}

pub(crate) async fn consume(conn: &mut PgConnection, id: InviteId) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        UPDATE invites
        SET used_at = now()
        WHERE id = $1
          AND used_at IS NULL
          AND expires_at > now()
        "#,
        id as InviteId,
    )
    .execute(conn)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub(crate) async fn delete(conn: &mut PgConnection, id: InviteId) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        "DELETE FROM invites WHERE id = $1 AND used_at IS NULL",
        id as InviteId
    )
    .execute(conn)
    .await?;
    Ok(result.rows_affected() == 1)
}
