use sqlx::PgConnection;

use crate::access::UserId;
use crate::auth::ids::SessionId;
use crate::auth::token::SecretHash;

#[derive(Debug)]
pub(crate) struct SessionRow {
    pub(crate) id: SessionId,
    pub(crate) user_id: UserId,
    pub(crate) username: String,
    pub(crate) display_name: String,
    pub(crate) is_admin: bool,
    pub(crate) is_stale: bool,
}

pub(crate) async fn insert(
    conn: &mut PgConnection,
    user_id: UserId,
    token: &SecretHash,
    lifetime_seconds: f64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO sessions (user_id, token_hash, expires_at)
        VALUES ($1, $2, now() + make_interval(secs => $3))
        "#,
        user_id as UserId,
        token.as_bytes(),
        lifetime_seconds,
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub(crate) async fn find_live(
    conn: &mut PgConnection,
    token: &SecretHash,
    idle_seconds: f64,
    touch_seconds: f64,
) -> Result<Option<SessionRow>, sqlx::Error> {
    sqlx::query_as!(
        SessionRow,
        r#"
        SELECT
          s.id AS "id: SessionId",
          u.id AS "user_id: UserId",
          u.username,
          u.display_name,
          u.is_admin,
          s.last_seen_at < now() - make_interval(secs => $3) AS "is_stale!"
        FROM sessions AS s
        JOIN users AS u ON u.id = s.user_id
        WHERE s.token_hash = $1
          AND s.expires_at > now()
          AND s.last_seen_at > now() - make_interval(secs => $2)
        "#,
        token.as_bytes(),
        idle_seconds,
        touch_seconds,
    )
    .fetch_optional(conn)
    .await
}

pub(crate) async fn touch(conn: &mut PgConnection, id: SessionId) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE sessions SET last_seen_at = now() WHERE id = $1",
        id as SessionId,
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub(crate) async fn delete(conn: &mut PgConnection, id: SessionId) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM sessions WHERE id = $1", id as SessionId)
        .execute(conn)
        .await?;
    Ok(())
}

pub(crate) async fn delete_for_user(
    conn: &mut PgConnection,
    user_id: UserId,
) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM sessions WHERE user_id = $1", user_id as UserId)
        .execute(conn)
        .await?;
    Ok(())
}

pub(crate) async fn delete_expired(conn: &mut PgConnection) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM sessions WHERE expires_at <= now()")
        .execute(conn)
        .await?;
    Ok(())
}
