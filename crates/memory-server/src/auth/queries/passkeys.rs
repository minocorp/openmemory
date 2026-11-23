use serde_json::Value;
use sqlx::PgConnection;
use time::OffsetDateTime;

use crate::access::UserId;
use crate::auth::ids::PasskeyId;

#[derive(Debug)]
pub(crate) struct PasskeyRow {
    pub(crate) id: PasskeyId,
    pub(crate) name: String,
    pub(crate) credential: Value,
    pub(crate) created_at: OffsetDateTime,
    pub(crate) last_used_at: Option<OffsetDateTime>,
}

pub(crate) async fn insert(
    conn: &mut PgConnection,
    user_id: UserId,
    credential_id: &[u8],
    credential: &Value,
    name: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO passkeys (user_id, credential_id, credential, name)
        VALUES ($1, $2, $3, $4)
        "#,
        user_id as UserId,
        credential_id,
        credential,
        name,
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub(crate) async fn list(
    conn: &mut PgConnection,
    user_id: UserId,
) -> Result<Vec<PasskeyRow>, sqlx::Error> {
    sqlx::query_as!(
        PasskeyRow,
        r#"
        SELECT id AS "id: PasskeyId", name, credential, created_at, last_used_at
        FROM passkeys
        WHERE user_id = $1
        ORDER BY created_at, id
        "#,
        user_id as UserId,
    )
    .fetch_all(conn)
    .await
}

pub(crate) async fn record_use(
    conn: &mut PgConnection,
    credential_id: &[u8],
    credential: &Value,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        UPDATE passkeys
        SET credential = $2, last_used_at = now()
        WHERE credential_id = $1
        "#,
        credential_id,
        credential,
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub(crate) async fn lock_for_user(
    conn: &mut PgConnection,
    user_id: UserId,
) -> Result<Vec<PasskeyId>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT id AS "id: PasskeyId"
        FROM passkeys
        WHERE user_id = $1
        ORDER BY id
        FOR UPDATE
        "#,
        user_id as UserId,
    )
    .fetch_all(conn)
    .await
}

pub(crate) async fn delete(
    conn: &mut PgConnection,
    user_id: UserId,
    id: PasskeyId,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        "DELETE FROM passkeys WHERE id = $1 AND user_id = $2",
        id as PasskeyId,
        user_id as UserId,
    )
    .execute(conn)
    .await?;
    Ok(result.rows_affected() == 1)
}

pub(crate) async fn delete_all_except(
    conn: &mut PgConnection,
    user_id: UserId,
    keep_credential_id: &[u8],
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query!(
        "DELETE FROM passkeys WHERE user_id = $1 AND credential_id <> $2",
        user_id as UserId,
        keep_credential_id,
    )
    .execute(conn)
    .await?;
    Ok(result.rows_affected())
}
