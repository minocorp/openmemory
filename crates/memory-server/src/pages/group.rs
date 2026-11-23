use sqlx::PgConnection;

use crate::access::Actor;
use crate::pages::error::PagesError;
use crate::pages::ids::GroupId;
use crate::pages::queries::groups;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum GroupJoin {
    Continue,
    New,
}

pub(super) async fn assign(
    conn: &mut PgConnection,
    actor: &Actor,
    task: Option<&str>,
    join: GroupJoin,
) -> Result<GroupId, PagesError> {
    groups::lock_writer(conn, writer_key(actor)).await?;
    if join == GroupJoin::Continue
        && let Some(open) = groups::find_open(conn, actor.user_id, actor.connection_id).await?
        && task.is_none_or(|label| open.task.as_deref().is_none_or(|current| current == label))
    {
        groups::touch(conn, open.id, task).await?;
        return Ok(open.id);
    }
    Ok(groups::insert(conn, actor.user_id, actor.connection_id, task).await?)
}

// Connections and users share one lock space: even keys are connections, odd keys are direct user sessions.
fn writer_key(actor: &Actor) -> i64 {
    match actor.connection_id {
        Some(connection) => connection.0.wrapping_mul(2),
        None => actor.user_id.0.wrapping_mul(2).wrapping_add(1),
    }
}
