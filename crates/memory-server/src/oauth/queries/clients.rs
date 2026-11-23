use sqlx::PgConnection;

use crate::oauth::ids::OauthClientId;

#[derive(Debug)]
pub(crate) struct ClientRow {
    pub(crate) id: OauthClientId,
    pub(crate) client_id: String,
    pub(crate) name: String,
    pub(crate) redirect_uris: Vec<String>,
}

pub(crate) async fn upsert(
    conn: &mut PgConnection,
    client_id: &str,
    name: &str,
    redirect_uris: &[String],
) -> Result<ClientRow, sqlx::Error> {
    sqlx::query_as!(
        ClientRow,
        r#"
        INSERT INTO oauth_clients (client_id, name, redirect_uris)
        VALUES ($1, $2, $3)
        ON CONFLICT (client_id) DO UPDATE
        SET name = excluded.name,
            redirect_uris = excluded.redirect_uris
        RETURNING id AS "id: OauthClientId", client_id, name, redirect_uris
        "#,
        client_id,
        name,
        redirect_uris,
    )
    .fetch_one(conn)
    .await
}

pub(crate) async fn find(
    conn: &mut PgConnection,
    client_id: &str,
) -> Result<Option<ClientRow>, sqlx::Error> {
    sqlx::query_as!(
        ClientRow,
        r#"
        SELECT id AS "id: OauthClientId", client_id, name, redirect_uris
        FROM oauth_clients
        WHERE client_id = $1
        "#,
        client_id,
    )
    .fetch_optional(conn)
    .await
}

pub(crate) async fn find_by_id(
    conn: &mut PgConnection,
    id: OauthClientId,
) -> Result<Option<ClientRow>, sqlx::Error> {
    sqlx::query_as!(
        ClientRow,
        r#"
        SELECT id AS "id: OauthClientId", client_id, name, redirect_uris
        FROM oauth_clients
        WHERE id = $1
        "#,
        id as OauthClientId,
    )
    .fetch_optional(conn)
    .await
}
