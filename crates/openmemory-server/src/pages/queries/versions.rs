use sqlx::PgConnection;
use time::OffsetDateTime;

use crate::access::Actor;
use crate::pages::ids::{GroupId, PageId, VersionNumber};

#[derive(Debug, Clone)]
pub(crate) struct VersionRow {
    pub(crate) n: VersionNumber,
    pub(crate) content: String,
    pub(crate) path: String,
    pub(crate) op: String,
    pub(crate) author_name: String,
    pub(crate) created_at: OffsetDateTime,
}

#[derive(Debug)]
pub(crate) struct NewVersion<'a> {
    pub(crate) page_id: PageId,
    pub(crate) n: VersionNumber,
    pub(crate) content: &'a str,
    pub(crate) path: &'a str,
    pub(crate) op: &'a str,
    pub(crate) reverts_version: Option<VersionNumber>,
    pub(crate) group_id: GroupId,
}

pub(crate) async fn insert(
    conn: &mut PgConnection,
    actor: &Actor,
    version: &NewVersion<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"INSERT INTO page_versions (
  page_id,
  n,
  content,
  path,
  op,
  reverts_version,
  user_id,
  connection_id,
  author_name,
  group_id
)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)"#,
        version.page_id as PageId,
        version.n as VersionNumber,
        version.content,
        version.path,
        version.op,
        version.reverts_version as Option<VersionNumber>,
        actor.user_id as crate::access::UserId,
        actor.connection_id as Option<crate::access::ConnectionId>,
        actor.name,
        version.group_id as GroupId
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub(crate) async fn find(
    conn: &mut PgConnection,
    page_id: PageId,
    n: VersionNumber,
) -> Result<Option<VersionRow>, sqlx::Error> {
    sqlx::query_as!(
        VersionRow,
        r#"SELECT
  n AS "n: VersionNumber",
  content,
  path,
  op,
  author_name,
  created_at
FROM page_versions
WHERE page_id = $1
  AND n = $2"#,
        page_id as PageId,
        n as VersionNumber
    )
    .fetch_optional(conn)
    .await
}
