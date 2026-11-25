use sqlx::PgConnection;

use crate::access::UserId;
use crate::auth::token::SecretHash;

pub(crate) async fn insert(
    conn: &mut PgConnection,
    user_id: UserId,
    code: &SecretHash,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO recovery_codes (user_id, code_hash) VALUES ($1, $2)",
        user_id as UserId,
        code.as_bytes(),
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub(crate) async fn is_unused(
    conn: &mut PgConnection,
    user_id: UserId,
    code: &SecretHash,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT EXISTS (
          SELECT 1
          FROM recovery_codes
          WHERE user_id = $1
            AND code_hash = $2
            AND used_at IS NULL
        ) AS "unused!"
        "#,
        user_id as UserId,
        code.as_bytes(),
    )
    .fetch_one(conn)
    .await
}

pub(crate) async fn consume(
    conn: &mut PgConnection,
    user_id: UserId,
    code: &SecretHash,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query!(
        r#"
        UPDATE recovery_codes
        SET used_at = now()
        WHERE user_id = $1
          AND code_hash = $2
          AND used_at IS NULL
        "#,
        user_id as UserId,
        code.as_bytes(),
    )
    .execute(conn)
    .await?;
    Ok(result.rows_affected() == 1)
}
