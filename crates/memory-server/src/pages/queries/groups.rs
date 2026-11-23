use sqlx::PgConnection;

use crate::access::{ConnectionId, UserId};
use crate::pages::ids::{GroupId, PageId, VersionNumber};

const GROUP_GAP_MINUTES: i32 = 30;

#[derive(Debug)]
pub(crate) struct OpenGroup {
    pub(crate) id: GroupId,
    pub(crate) task: Option<String>,
}

#[derive(Debug)]
pub(crate) struct GroupPage {
    pub(crate) page_id: PageId,
    pub(crate) first: VersionNumber,
    pub(crate) last: VersionNumber,
}

pub(crate) async fn lock_writer(conn: &mut PgConnection, key: i64) -> Result<(), sqlx::Error> {
    sqlx::query!("SELECT pg_advisory_xact_lock($1)", key)
        .execute(conn)
        .await?;
    Ok(())
}

pub(crate) async fn find_open(
    conn: &mut PgConnection,
    user_id: UserId,
    connection_id: Option<ConnectionId>,
) -> Result<Option<OpenGroup>, sqlx::Error> {
    sqlx::query_as!(
        OpenGroup,
        r#"SELECT
  id AS "id: GroupId",
  task
FROM change_groups
WHERE user_id = $1
  AND connection_id IS NOT DISTINCT FROM $2
  AND last_at > now() - make_interval(mins => $3)
ORDER BY last_at DESC, id DESC
LIMIT 1"#,
        user_id as UserId,
        connection_id as Option<ConnectionId>,
        GROUP_GAP_MINUTES
    )
    .fetch_optional(conn)
    .await
}

pub(crate) async fn touch(
    conn: &mut PgConnection,
    id: GroupId,
    task: Option<&str>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"UPDATE change_groups
SET last_at = GREATEST(last_at, now()),
  task = COALESCE(task, $2)
WHERE id = $1"#,
        id as GroupId,
        task
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub(crate) async fn insert(
    conn: &mut PgConnection,
    user_id: UserId,
    connection_id: Option<ConnectionId>,
    task: Option<&str>,
) -> Result<GroupId, sqlx::Error> {
    sqlx::query_scalar!(
        r#"INSERT INTO change_groups (user_id, connection_id, task)
VALUES ($1, $2, $3)
RETURNING id AS "id: GroupId""#,
        user_id as UserId,
        connection_id as Option<ConnectionId>,
        task
    )
    .fetch_one(conn)
    .await
}

pub(crate) async fn pages_in_group(
    conn: &mut PgConnection,
    id: GroupId,
) -> Result<Vec<GroupPage>, sqlx::Error> {
    sqlx::query_as!(
        GroupPage,
        r#"SELECT
  page_id AS "page_id: PageId",
  MIN(n) AS "first!: VersionNumber",
  MAX(n) AS "last!: VersionNumber"
FROM page_versions
WHERE group_id = $1
GROUP BY page_id
ORDER BY page_id"#,
        id as GroupId
    )
    .fetch_all(conn)
    .await
}
