use sqlx::PgConnection;

use crate::access::UserId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Subject {
    Page,
    File,
}

impl Subject {
    fn as_str(self) -> &'static str {
        match self {
            Self::Page => "page",
            Self::File => "file",
        }
    }
}

pub(crate) async fn record(
    conn: &mut PgConnection,
    subject: Subject,
    id: i64,
    path: &str,
    user_id: UserId,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "INSERT INTO purges (kind, subject_id, path, user_id) VALUES ($1, $2, $3, $4)",
        subject.as_str(),
        id,
        path,
        user_id as UserId
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub(crate) async fn is_purged(
    conn: &mut PgConnection,
    subject: Subject,
    id: i64,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar!(
        r#"SELECT EXISTS (
  SELECT 1
  FROM purges
  WHERE kind = $1
    AND subject_id = $2
) AS "purged!""#,
        subject.as_str(),
        id
    )
    .fetch_one(conn)
    .await
}
