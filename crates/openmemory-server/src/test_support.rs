use openmemory_ops::{PagePath, WriteAction, WriteOutput};
use sqlx::PgPool;

use crate::access::{Actor, ConnectionId, Folders, Scope, UserId};
use crate::pages::write::write;
use crate::time_zone::TimeZone;

pub(crate) fn path(value: &str) -> PagePath {
    PagePath::parse(value).unwrap()
}

pub(crate) async fn add_user(pool: &PgPool, name: &str) -> UserId {
    sqlx::query_scalar!(
        r#"INSERT INTO users (username, display_name, user_handle)
VALUES ($1, $1, uuid_send(gen_random_uuid()))
RETURNING id AS "id: UserId""#,
        name
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

pub(crate) async fn add_connection(pool: &PgPool, user: UserId, name: &str) -> ConnectionId {
    sqlx::query_scalar!(
        r#"INSERT INTO connections (user_id, name, scopes)
VALUES ($1, $2, ARRAY['/'])
RETURNING id AS "id: ConnectionId""#,
        user as UserId,
        name
    )
    .fetch_one(pool)
    .await
    .unwrap()
}

pub(crate) async fn agent(pool: &PgPool, user_name: &str, connection_name: &str) -> Actor {
    let user_id = add_user(pool, user_name).await;
    let connection_id = add_connection(pool, user_id, connection_name).await;
    Actor {
        user_id,
        connection_id: Some(connection_id),
        name: format!("{user_name}/{connection_name}"),
        scope: Scope::everything(),
    }
}

pub(crate) async fn another_connection(pool: &PgPool, actor: &Actor, name: &str) -> Actor {
    let connection_id = add_connection(pool, actor.user_id, name).await;
    Actor {
        connection_id: Some(connection_id),
        name: format!("{}/{name}", actor.name),
        ..actor.clone()
    }
}

pub(crate) fn scoped(actor: &Actor, folders: &[&str], can_write: bool) -> Actor {
    Actor {
        scope: Scope {
            folders: Folders::Only(folders.iter().map(|folder| path(folder)).collect()),
            can_write,
        },
        ..actor.clone()
    }
}

pub(crate) async fn create(
    pool: &PgPool,
    actor: &Actor,
    page_path: &str,
    content: &str,
) -> WriteOutput {
    write(
        pool,
        &TimeZone::utc(),
        actor,
        &path(page_path),
        WriteAction::Create {
            content: content.to_owned(),
        },
        None,
    )
    .await
    .unwrap()
}

pub(crate) async fn age_groups(pool: &PgPool) {
    sqlx::query!(
        "UPDATE change_groups SET created_at = created_at - interval '31 minutes', last_at = last_at - interval '31 minutes'"
    )
    .execute(pool)
    .await
    .unwrap();
}
