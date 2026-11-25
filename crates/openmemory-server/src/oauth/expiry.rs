use sqlx::PgPool;

use crate::access::connections;
use crate::oauth::device::DEVICE_SECONDS;
use crate::oauth::queries::{consents, devices};

// An approved connection can only be exchanged while its device code lives (the longest grant), so older ones are abandoned.
const UNEXCHANGED_GRACE_SECONDS: f64 = DEVICE_SECONDS + 60.0;

pub(crate) async fn expire_pending(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let mut tx = pool.begin().await?;
    consents::delete_expired(&mut tx).await?;
    devices::delete_expired(&mut tx).await?;
    let removed = connections::delete_unexchanged(&mut tx, UNEXCHANGED_GRACE_SECONDS).await?;
    tx.commit().await?;
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access::{Audience, ConnectionId, Folders, Name, UserId};
    use crate::auth::token::Secret;
    use crate::oauth::ids::OauthClientId;
    use crate::oauth::queries::clients;
    use crate::oauth::queries::tokens;

    async fn connection(pool: &PgPool, user: UserId, name: &str, age_minutes: i32) -> ConnectionId {
        let mut conn = pool.acquire().await.unwrap();
        let id = connections::create(
            &mut conn,
            user,
            &Name::parse(name).unwrap(),
            &Folders::All,
            true,
            Audience::Mcp,
        )
        .await
        .unwrap();
        sqlx::query!(
            "UPDATE connections SET created_at = now() - make_interval(mins => $2) WHERE id = $1",
            id as ConnectionId,
            age_minutes,
        )
        .execute(&mut *conn)
        .await
        .unwrap();
        id
    }

    async fn exists(pool: &PgPool, id: ConnectionId) -> bool {
        sqlx::query_scalar!(
            r#"SELECT EXISTS (SELECT 1 FROM connections WHERE id = $1) AS "exists!""#,
            id as ConnectionId
        )
        .fetch_one(pool)
        .await
        .unwrap()
    }

    #[sqlx::test]
    async fn abandoned_approvals_are_removed_and_exchanged_ones_kept(pool: PgPool) {
        let user = UserId(
            sqlx::query_scalar!("INSERT INTO users (username, display_name, user_handle) VALUES ('mino', 'Mino', uuid_send(gen_random_uuid())) RETURNING id")
                .fetch_one(&pool)
                .await
                .unwrap(),
        );
        let client: OauthClientId = {
            let mut conn = pool.acquire().await.unwrap();
            clients::upsert(
                &mut conn,
                "dcr-test",
                "Test",
                &["https://a.example/cb".to_owned()],
            )
            .await
            .unwrap()
            .id
        };
        let abandoned = connection(&pool, user, "abandoned", 30).await;
        let fresh = connection(&pool, user, "fresh", 1).await;
        let exchanged = connection(&pool, user, "exchanged", 30).await;
        let mut conn = pool.acquire().await.unwrap();
        tokens::insert(
            &mut conn,
            exchanged,
            client,
            &Secret::generate().hash(),
            &Secret::generate().hash(),
            3600.0,
            86400.0,
        )
        .await
        .unwrap();
        drop(conn);
        let live = Secret::generate();
        let stale = Secret::generate();
        let mut conn = pool.acquire().await.unwrap();
        for (secret, lifetime) in [(&live, 600.0), (&stale, 600.0)] {
            consents::insert(
                &mut conn,
                &consents::NewConsent {
                    token: &secret.hash(),
                    oauth_client_id: client,
                    redirect_uri: "https://a.example/cb",
                    code_challenge: &"c".repeat(43),
                    client_state: None,
                    audience: Audience::Mcp,
                    lifetime_seconds: lifetime,
                },
            )
            .await
            .unwrap();
        }
        drop(conn);
        let stale_hash = stale.hash();
        sqlx::query!(
            "UPDATE oauth_consents SET created_at = now() - interval '2 hours', expires_at = now() - interval '1 hour' WHERE token_hash = $1",
            stale_hash.as_bytes(),
        )
        .execute(&pool)
        .await
        .unwrap();

        let removed = expire_pending(&pool).await.unwrap();

        assert_eq!(removed, 1);
        assert!(!exists(&pool, abandoned).await);
        assert!(exists(&pool, fresh).await);
        assert!(exists(&pool, exchanged).await);
        let mut conn = pool.acquire().await.unwrap();
        assert!(
            consents::find(&mut conn, &live.hash())
                .await
                .unwrap()
                .is_some()
        );
        let remaining = sqlx::query_scalar!(r#"SELECT count(*) AS "count!" FROM oauth_consents"#)
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(remaining, 1);
    }
}
