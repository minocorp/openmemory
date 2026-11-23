use sqlx::PgConnection;
use time::OffsetDateTime;

use crate::access::{ConnectionId, UserId};

#[derive(Debug)]
pub(crate) struct ConnectionRow {
    pub(crate) id: ConnectionId,
    pub(crate) user_id: UserId,
    pub(crate) owner: String,
    pub(crate) owner_display_name: String,
    pub(crate) name: String,
    pub(crate) scopes: Vec<String>,
    pub(crate) can_write: bool,
    pub(crate) created_at: OffsetDateTime,
    pub(crate) last_used_at: Option<OffsetDateTime>,
}

pub(crate) async fn insert(
    conn: &mut PgConnection,
    user_id: UserId,
    name: &str,
    scopes: &[String],
    can_write: bool,
) -> Result<ConnectionId, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        INSERT INTO connections (user_id, name, scopes, can_write)
        VALUES ($1, $2, $3, $4)
        RETURNING id AS "id: ConnectionId"
        "#,
        user_id as UserId,
        name,
        scopes,
        can_write,
    )
    .fetch_one(conn)
    .await
}

pub(crate) async fn list_live(
    conn: &mut PgConnection,
    only_user: Option<UserId>,
) -> Result<Vec<ConnectionRow>, sqlx::Error> {
    sqlx::query_as!(
        ConnectionRow,
        r#"
        SELECT
          c.id AS "id: ConnectionId",
          c.user_id AS "user_id: UserId",
          u.username AS owner,
          u.display_name AS owner_display_name,
          c.name,
          c.scopes,
          c.can_write,
          c.created_at,
          c.last_used_at
        FROM connections AS c
        JOIN users AS u ON u.id = c.user_id
        WHERE c.revoked_at IS NULL
          AND ($1::bigint IS NULL OR c.user_id = $1)
        ORDER BY u.username, c.name, c.id
        "#,
        only_user as Option<UserId>,
    )
    .fetch_all(conn)
    .await
}

pub(crate) async fn find_live(
    conn: &mut PgConnection,
    id: ConnectionId,
) -> Result<Option<ConnectionRow>, sqlx::Error> {
    sqlx::query_as!(
        ConnectionRow,
        r#"
        SELECT
          c.id AS "id: ConnectionId",
          c.user_id AS "user_id: UserId",
          u.username AS owner,
          u.display_name AS owner_display_name,
          c.name,
          c.scopes,
          c.can_write,
          c.created_at,
          c.last_used_at
        FROM connections AS c
        JOIN users AS u ON u.id = c.user_id
        WHERE c.id = $1
          AND c.revoked_at IS NULL
        "#,
        id as ConnectionId,
    )
    .fetch_optional(conn)
    .await
}

pub(crate) async fn touch(conn: &mut PgConnection, id: ConnectionId) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE connections
        SET last_used_at = now()
        WHERE id = $1
          AND (last_used_at IS NULL OR last_used_at < now() - interval '1 minute')
        "#,
        id as ConnectionId,
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub(crate) async fn update(
    conn: &mut PgConnection,
    id: ConnectionId,
    name: Option<&str>,
    scopes: Option<&[String]>,
    can_write: Option<bool>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE connections
        SET name = COALESCE($2, name),
            scopes = COALESCE($3, scopes),
            can_write = COALESCE($4, can_write)
        WHERE id = $1
          AND revoked_at IS NULL
        "#,
        id as ConnectionId,
        name,
        scopes,
        can_write,
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub(crate) async fn revoke(conn: &mut PgConnection, id: ConnectionId) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE connections SET revoked_at = now() WHERE id = $1 AND revoked_at IS NULL",
        id as ConnectionId,
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub(crate) async fn delete_unexchanged(
    conn: &mut PgConnection,
    older_than_seconds: f64,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        DELETE FROM connections AS c
        WHERE c.revoked_at IS NULL
          AND c.created_at < now() - make_interval(secs => $1)
          AND NOT EXISTS (
            SELECT 1
            FROM oauth_tokens AS t
            WHERE t.connection_id = c.id
          )
        "#,
        older_than_seconds,
    )
    .execute(conn)
    .await?;
    Ok(result.rows_affected())
}

pub(crate) async fn revoke_all_for_user(
    conn: &mut PgConnection,
    user_id: UserId,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "UPDATE connections SET revoked_at = now() WHERE user_id = $1 AND revoked_at IS NULL",
        user_id as UserId,
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub(crate) async fn name_taken(
    conn: &mut PgConnection,
    user_id: UserId,
    name: &str,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM connections
          WHERE user_id = $1
            AND name = $2
            AND revoked_at IS NULL
        ) AS "taken!"
        "#,
        user_id as UserId,
        name,
    )
    .fetch_one(conn)
    .await
}
