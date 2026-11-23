use sqlx::PgConnection;

use crate::access::UserId;

const FIRST_USER_LOCK: i64 = 0x6e6f_7465_7301;

#[derive(Debug)]
pub(crate) struct UserRow {
    pub(crate) id: UserId,
    pub(crate) username: String,
    pub(crate) display_name: String,
    pub(crate) user_handle: Vec<u8>,
}

#[derive(Debug)]
pub(crate) struct UserSummary {
    pub(crate) id: UserId,
    pub(crate) username: String,
    pub(crate) display_name: String,
    pub(crate) is_admin: bool,
    pub(crate) created_at: time::OffsetDateTime,
}

pub(crate) async fn list(conn: &mut PgConnection) -> Result<Vec<UserSummary>, sqlx::Error> {
    sqlx::query_as!(
        UserSummary,
        r#"
        SELECT id AS "id: UserId", username, display_name, is_admin, created_at
        FROM users
        ORDER BY id
        "#,
    )
    .fetch_all(conn)
    .await
}

pub(crate) async fn lock_admins(conn: &mut PgConnection) -> Result<Vec<UserId>, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        SELECT id AS "id: UserId"
        FROM users
        WHERE is_admin
        ORDER BY id
        FOR UPDATE
        "#,
    )
    .fetch_all(conn)
    .await
}

pub(crate) async fn set_admin(
    conn: &mut PgConnection,
    id: UserId,
    is_admin: bool,
) -> Result<Option<UserSummary>, sqlx::Error> {
    sqlx::query_as!(
        UserSummary,
        r#"
        UPDATE users
        SET is_admin = $2
        WHERE id = $1
        RETURNING id AS "id: UserId", username, display_name, is_admin, created_at
        "#,
        id as UserId,
        is_admin,
    )
    .fetch_optional(conn)
    .await
}

pub(crate) async fn lock_first_user(conn: &mut PgConnection) -> Result<(), sqlx::Error> {
    sqlx::query!("SELECT pg_advisory_xact_lock($1)", FIRST_USER_LOCK)
        .fetch_one(conn)
        .await?;
    Ok(())
}

pub(crate) async fn any_exists(conn: &mut PgConnection) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(r#"SELECT EXISTS (SELECT 1 FROM users) AS "exists!""#)
        .fetch_one(conn)
        .await
}

pub(crate) async fn insert(
    conn: &mut PgConnection,
    username: &str,
    display_name: &str,
    user_handle: &[u8],
    is_admin: bool,
) -> Result<UserId, sqlx::Error> {
    sqlx::query_scalar!(
        r#"
        INSERT INTO users (username, display_name, user_handle, is_admin)
        VALUES ($1, $2, $3, $4)
        RETURNING id AS "id: UserId"
        "#,
        username,
        display_name,
        user_handle,
        is_admin,
    )
    .fetch_one(conn)
    .await
}

pub(crate) async fn find_by_username(
    conn: &mut PgConnection,
    username: &str,
) -> Result<Option<UserRow>, sqlx::Error> {
    sqlx::query_as!(
        UserRow,
        r#"
        SELECT id AS "id: UserId", username, display_name, user_handle
        FROM users
        WHERE username = $1
        "#,
        username,
    )
    .fetch_optional(conn)
    .await
}

pub(crate) async fn find_by_user_handle(
    conn: &mut PgConnection,
    user_handle: &[u8],
) -> Result<Option<UserRow>, sqlx::Error> {
    sqlx::query_as!(
        UserRow,
        r#"
        SELECT id AS "id: UserId", username, display_name, user_handle
        FROM users
        WHERE user_handle = $1
        "#,
        user_handle,
    )
    .fetch_optional(conn)
    .await
}

pub(crate) async fn find(
    conn: &mut PgConnection,
    id: UserId,
) -> Result<Option<UserRow>, sqlx::Error> {
    sqlx::query_as!(
        UserRow,
        r#"
        SELECT id AS "id: UserId", username, display_name, user_handle
        FROM users
        WHERE id = $1
        "#,
        id as UserId,
    )
    .fetch_optional(conn)
    .await
}

pub(crate) async fn update_profile(
    conn: &mut PgConnection,
    id: UserId,
    username: Option<&str>,
    display_name: Option<&str>,
) -> Result<Option<UserSummary>, sqlx::Error> {
    sqlx::query_as!(
        UserSummary,
        r#"
        UPDATE users
        SET username = COALESCE($2, username),
            display_name = COALESCE($3, display_name)
        WHERE id = $1
        RETURNING id AS "id: UserId", username, display_name, is_admin, created_at
        "#,
        id as UserId,
        username,
        display_name,
    )
    .fetch_optional(conn)
    .await
}
