use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode};
use axum::routing::get;
use object_store::memory::InMemory;
use sqlx::PgPool;
use tower::ServiceExt;

use super::McpActor;
use crate::access::{Folders, Name, UserId, connections};
use crate::auth::token::Secret;
use crate::oauth::ids::OauthClientId;
use crate::oauth::queries::{clients, tokens};
use crate::state::AppState;

const METADATA: &str =
    r#"Bearer resource_metadata="http://localhost:8080/.well-known/oauth-protected-resource/mcp""#;

struct Fixture {
    app: Router,
    access: Secret,
    connection_id: crate::access::ConnectionId,
}

async fn fixture(pool: PgPool) -> Fixture {
    let mut conn = pool.acquire().await.unwrap();
    let user_id: UserId =
        sqlx::query_scalar("INSERT INTO users (username, display_name, user_handle) VALUES ('mino', 'Mino', uuid_send(gen_random_uuid())) RETURNING id")
            .fetch_one(&mut *conn)
            .await
            .unwrap();
    let name = Name::parse("claude-code").unwrap();
    let folders = Folders::Only(vec![memory_ops::PagePath::parse("tax").unwrap()]);
    let connection_id = connections::create(&mut conn, user_id, &name, &folders, false)
        .await
        .unwrap();
    let client = clients::upsert(
        &mut conn,
        "dcr-test",
        "Test",
        &["https://a.example/cb".to_owned()],
    )
    .await
    .unwrap();
    let access = Secret::generate();
    insert_token(&mut conn, connection_id, client.id, &access, 3600.0).await;
    drop(conn);
    let state = AppState::new(pool.clone(), Arc::new(InMemory::new()), 1024);
    let app = Router::new()
        .route(
            "/whoami",
            get(|McpActor(actor): McpActor| async move {
                format!(
                    "{} {:?} {}",
                    actor.name, actor.scope.folders, actor.scope.can_write
                )
            }),
        )
        .with_state(state);
    Fixture {
        app,
        access,
        connection_id,
    }
}

async fn insert_token(
    conn: &mut sqlx::PgConnection,
    connection_id: crate::access::ConnectionId,
    client: OauthClientId,
    access: &Secret,
    access_seconds: f64,
) {
    tokens::insert(
        conn,
        connection_id,
        client,
        &access.hash(),
        &Secret::generate().hash(),
        access_seconds,
        access_seconds + 10.0,
    )
    .await
    .unwrap();
}

async fn call(app: &Router, authorization: Option<&str>) -> (StatusCode, Option<String>, String) {
    let mut request = Request::get("/whoami");
    if let Some(value) = authorization {
        request = request.header("authorization", value);
    }
    let response = app
        .clone()
        .oneshot(request.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let status = response.status();
    let challenge = response
        .headers()
        .get("www-authenticate")
        .map(|value| value.to_str().unwrap().to_owned());
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    (status, challenge, String::from_utf8(body.to_vec()).unwrap())
}

#[sqlx::test]
async fn a_valid_token_yields_the_connections_actor(pool: PgPool) {
    let fixture = fixture(pool).await;
    let header = format!("Bearer {}", fixture.access.expose());
    let (status, _, body) = call(&fixture.app, Some(&header)).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body.starts_with("mino/claude-code Only("), "{body}");
    assert!(body.ends_with(" false"), "{body}");
    let lowercase = format!("bearer {}", fixture.access.expose());
    assert_eq!(call(&fixture.app, Some(&lowercase)).await.0, StatusCode::OK);
}

#[sqlx::test]
async fn a_missing_token_is_challenged_with_the_metadata_url(pool: PgPool) {
    let fixture = fixture(pool).await;
    let (status, challenge, _) = call(&fixture.app, None).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(challenge.as_deref(), Some(METADATA));
}

#[sqlx::test]
async fn bad_tokens_are_challenged_as_invalid(pool: PgPool) {
    let fixture = fixture(pool).await;
    for header in [
        "Bearer nonsense",
        "Basic bWlubzpwdw==",
        "Bearer",
        "nonsense",
    ] {
        let (status, challenge, _) = call(&fixture.app, Some(header)).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{header}");
        let challenge = challenge.unwrap();
        assert!(challenge.contains(r#"error="invalid_token""#), "{header}");
        assert!(challenge.contains("resource_metadata="), "{header}");
    }
}

#[sqlx::test]
async fn expired_access_tokens_are_refused(pool: PgPool) {
    let fixture = fixture(pool.clone()).await;
    sqlx::query(
        "UPDATE oauth_tokens SET created_at = now() - interval '2 hours', access_expires_at = now() - interval '1 hour'",
    )
    .execute(&pool)
    .await
    .unwrap();
    let header = format!("Bearer {}", fixture.access.expose());
    assert_eq!(
        call(&fixture.app, Some(&header)).await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[sqlx::test]
async fn revoking_the_connection_fails_the_next_request(pool: PgPool) {
    let fixture = fixture(pool.clone()).await;
    let header = format!("Bearer {}", fixture.access.expose());
    assert_eq!(call(&fixture.app, Some(&header)).await.0, StatusCode::OK);
    let mut conn = pool.acquire().await.unwrap();
    connections::revoke_one(&mut conn, fixture.connection_id)
        .await
        .unwrap();
    assert_eq!(
        call(&fixture.app, Some(&header)).await.0,
        StatusCode::UNAUTHORIZED
    );
}

#[sqlx::test]
async fn using_the_token_records_when_the_connection_was_last_used(pool: PgPool) {
    let fixture = fixture(pool.clone()).await;
    let used = || async {
        sqlx::query_scalar::<_, Option<time::OffsetDateTime>>(
            "SELECT last_used_at FROM connections WHERE id = $1",
        )
        .bind(fixture.connection_id)
        .fetch_one(&pool)
        .await
        .unwrap()
    };
    assert!(used().await.is_none());
    let header = format!("Bearer {}", fixture.access.expose());
    assert_eq!(call(&fixture.app, Some(&header)).await.0, StatusCode::OK);
    assert!(used().await.is_some());
}
