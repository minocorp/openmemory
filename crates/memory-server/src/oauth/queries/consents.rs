use sqlx::PgConnection;

use crate::access::Audience;
use crate::auth::token::SecretHash;
use crate::oauth::ids::OauthClientId;

#[derive(Debug)]
pub(crate) struct ConsentRow {
    pub(crate) oauth_client_id: OauthClientId,
    pub(crate) redirect_uri: String,
    pub(crate) code_challenge: String,
    pub(crate) client_state: Option<String>,
    pub(crate) audience: Audience,
}

pub(crate) struct NewConsent<'a> {
    pub(crate) token: &'a SecretHash,
    pub(crate) oauth_client_id: OauthClientId,
    pub(crate) redirect_uri: &'a str,
    pub(crate) code_challenge: &'a str,
    pub(crate) client_state: Option<&'a str>,
    pub(crate) audience: Audience,
    pub(crate) lifetime_seconds: f64,
}

pub(crate) async fn insert(
    conn: &mut PgConnection,
    consent: &NewConsent<'_>,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        r#"
        INSERT INTO oauth_consents
          (token_hash, oauth_client_id, redirect_uri, code_challenge, client_state, audience, expires_at)
        VALUES ($1, $2, $3, $4, $5, $6, now() + make_interval(secs => $7))
        "#,
        consent.token.as_bytes(),
        consent.oauth_client_id as OauthClientId,
        consent.redirect_uri,
        consent.code_challenge,
        consent.client_state,
        consent.audience as Audience,
        consent.lifetime_seconds,
    )
    .execute(conn)
    .await?;
    Ok(())
}

pub(crate) async fn find(
    conn: &mut PgConnection,
    token: &SecretHash,
) -> Result<Option<ConsentRow>, sqlx::Error> {
    sqlx::query_as!(
        ConsentRow,
        r#"
        SELECT
          oauth_client_id AS "oauth_client_id: OauthClientId",
          redirect_uri,
          code_challenge,
          client_state,
          audience AS "audience: Audience"
        FROM oauth_consents
        WHERE token_hash = $1
          AND expires_at > now()
        "#,
        token.as_bytes(),
    )
    .fetch_optional(conn)
    .await
}

pub(crate) async fn take(
    conn: &mut PgConnection,
    token: &SecretHash,
) -> Result<Option<ConsentRow>, sqlx::Error> {
    sqlx::query_as!(
        ConsentRow,
        r#"
        DELETE FROM oauth_consents
        WHERE token_hash = $1
          AND expires_at > now()
        RETURNING
          oauth_client_id AS "oauth_client_id: OauthClientId",
          redirect_uri,
          code_challenge,
          client_state,
          audience AS "audience: Audience"
        "#,
        token.as_bytes(),
    )
    .fetch_optional(conn)
    .await
}

pub(crate) async fn delete_expired(conn: &mut PgConnection) -> Result<(), sqlx::Error> {
    sqlx::query!("DELETE FROM oauth_consents WHERE expires_at <= now()")
        .execute(conn)
        .await?;
    Ok(())
}
