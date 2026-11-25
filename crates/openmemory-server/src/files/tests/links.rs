use super::*;

#[sqlx::test]
async fn a_link_uploads_once_as_its_connection(pool: PgPool) {
    let fx = fixture(pool).await;
    let link = fx
        .files
        .create_upload_link(&fx.actor, "scans/receipt.png")
        .await
        .unwrap();
    assert_eq!(link.token.len(), 64);
    let record = fx
        .files
        .redeem_upload_link(&link.token, Bytes::from_static(b"png"), "image/png")
        .await
        .unwrap();
    assert_eq!(record.path.as_str(), "scans/receipt.png");
    assert_eq!(record.uploader, fx.actor.user_id);
    let again = fx
        .files
        .redeem_upload_link(&link.token, Bytes::from_static(b"png"), "image/png")
        .await;
    assert!(matches!(again, Err(FilesError::LinkInvalid)));
}

#[sqlx::test]
async fn only_the_token_hash_is_stored(pool: PgPool) {
    let fx = fixture(pool).await;
    let link = fx
        .files
        .create_upload_link(&fx.actor, "a.png")
        .await
        .unwrap();
    let stored = sqlx::query_scalar!("SELECT token_hash FROM upload_links")
        .fetch_one(&fx.pool)
        .await
        .unwrap();
    assert_eq!(stored, Sha256Hash::of(link.token.as_bytes()).as_bytes());
}

#[sqlx::test]
async fn the_link_expires_after_fifteen_minutes(pool: PgPool) {
    let fx = fixture(pool).await;
    let link = fx
        .files
        .create_upload_link(&fx.actor, "a.png")
        .await
        .unwrap();
    let lifetime_seconds = sqlx::query_scalar!(
        r#"SELECT extract(epoch FROM expires_at - created_at)::bigint AS "seconds!" FROM upload_links"#
    )
    .fetch_one(&fx.pool)
    .await
    .unwrap();
    assert_eq!(lifetime_seconds, 15 * 60);
    sqlx::query!(
        "UPDATE upload_links SET created_at = now() - interval '1 hour', expires_at = now() - interval '1 second'"
    )
    .execute(&fx.pool)
    .await
    .unwrap();
    let late = fx
        .files
        .redeem_upload_link(&link.token, Bytes::from_static(b"png"), "image/png")
        .await;
    assert!(matches!(late, Err(FilesError::LinkInvalid)));
}

#[sqlx::test]
async fn an_unknown_token_is_invalid(pool: PgPool) {
    let fx = fixture(pool).await;
    let result = fx
        .files
        .redeem_upload_link("nope", Bytes::from_static(b"png"), "image/png")
        .await;
    assert!(matches!(result, Err(FilesError::LinkInvalid)));
}

#[sqlx::test]
async fn a_failed_upload_leaves_the_link_usable(pool: PgPool) {
    let fx = fixture(pool).await;
    let link = fx
        .files
        .create_upload_link(&fx.actor, "a.png")
        .await
        .unwrap();
    let rejected = fx
        .files
        .redeem_upload_link(&link.token, Bytes::from_static(b"x"), "text/html")
        .await;
    assert!(matches!(rejected, Err(FilesError::UnsupportedMime)));
    let retried = fx
        .files
        .redeem_upload_link(&link.token, Bytes::from_static(b"png"), "image/png")
        .await;
    assert!(retried.is_ok());
}

#[sqlx::test]
async fn a_link_needs_a_connection_and_write_access(pool: PgPool) {
    let fx = fixture(pool).await;
    let person = Actor {
        connection_id: None,
        ..fx.actor.clone()
    };
    let without_connection = fx.files.create_upload_link(&person, "a.png").await;
    assert!(matches!(
        without_connection,
        Err(FilesError::ConnectionRequired)
    ));
    let read_only = Actor {
        scope: Scope {
            can_write: false,
            ..Scope::everything()
        },
        ..fx.actor.clone()
    };
    let forbidden = fx.files.create_upload_link(&read_only, "a.png").await;
    assert!(matches!(forbidden, Err(FilesError::Forbidden)));
}

#[sqlx::test]
async fn redeeming_uses_the_connection_scope_and_revocation(pool: PgPool) {
    let fx = fixture(pool).await;
    let scoped = insert_connection(&fx.pool, fx.actor.user_id.0, "tax-agent", "tax").await;
    let scoped_actor = Actor {
        connection_id: Some(ConnectionId(scoped)),
        ..fx.actor.clone()
    };
    let inside = fx
        .files
        .create_upload_link(&scoped_actor, "tax/a.png")
        .await;
    assert!(inside.is_ok());
    sqlx::query!("UPDATE upload_links SET path = 'legal/a.png'")
        .execute(&fx.pool)
        .await
        .unwrap();
    let token = inside.unwrap().token;
    let outside = fx
        .files
        .redeem_upload_link(&token, Bytes::from_static(b"png"), "image/png")
        .await;
    assert!(matches!(outside, Err(FilesError::Forbidden)));
    sqlx::query!(
        "UPDATE connections SET revoked_at = now() WHERE id = $1",
        scoped
    )
    .execute(&fx.pool)
    .await
    .unwrap();
    let revoked = fx
        .files
        .redeem_upload_link(&token, Bytes::from_static(b"png"), "image/png")
        .await;
    assert!(matches!(revoked, Err(FilesError::LinkInvalid)));
}
