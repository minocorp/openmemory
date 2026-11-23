use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use crate::auth::queries::users;
use crate::auth::token::{Secret, SecretHash};
use crate::state::AppState;

const LIFETIME: Duration = Duration::from_hours(24);

#[derive(Debug, Clone, Default)]
pub(crate) struct Bootstrap {
    token: Arc<Mutex<Option<(SecretHash, Instant)>>>,
}

impl Bootstrap {
    fn arm(&self) -> Secret {
        self.arm_for(LIFETIME)
    }

    fn arm_for(&self, lifetime: Duration) -> Secret {
        let secret = Secret::generate();
        *self.token.lock().unwrap_or_else(PoisonError::into_inner) =
            Some((secret.hash(), Instant::now() + lifetime));
        secret
    }

    pub(crate) fn accepts(&self, secret: &Secret) -> bool {
        self.token
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .as_ref()
            .is_some_and(|(expected, expires_at)| {
                Instant::now() < *expires_at && expected.matches(&secret.hash())
            })
    }

    pub(crate) fn disarm(&self) {
        *self.token.lock().unwrap_or_else(PoisonError::into_inner) = None;
    }
}

pub(crate) async fn arm_if_no_users(state: &AppState) -> Result<Option<String>, sqlx::Error> {
    let mut conn = state.pool().acquire().await?;
    if users::any_exists(&mut conn).await? {
        return Ok(None);
    }
    let secret = state.bootstrap().arm();
    Ok(Some(
        state
            .public_url()
            .join(&format!("/bootstrap?token={}", secret.expose())),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_token_is_accepted_until_it_expires() {
        let bootstrap = Bootstrap::default();
        let secret = bootstrap.arm();
        assert!(bootstrap.accepts(&secret));
        let expired = bootstrap.arm_for(Duration::ZERO);
        assert!(!bootstrap.accepts(&expired));
        assert!(!bootstrap.accepts(&secret));
    }
}
