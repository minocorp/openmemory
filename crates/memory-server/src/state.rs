use std::sync::Arc;

use object_store::ObjectStore;
use sqlx::PgPool;
use webauthn_rs::prelude::WebauthnError;

use crate::auth::{Bootstrap, Passkeys};
use crate::backup::Backup;
use crate::config::PublicUrl;
use crate::files::Files;
use crate::oauth::ClientFetcher;
use crate::rate_limit::{RateLimits, TrustedProxy};
use crate::time_zone::TimeZone;

#[derive(Clone)]
pub struct AppState {
    pool: PgPool,
    files: Files,
    public_url: PublicUrl,
    passkeys: Passkeys,
    bootstrap: Bootstrap,
    fetcher: ClientFetcher,
    time_zone: TimeZone,
    rate_limits: RateLimits,
    backup: Backup,
}

impl AppState {
    pub fn new(pool: PgPool, store: Arc<dyn ObjectStore>, max_upload_bytes: u64) -> Self {
        let files = Files::new(pool.clone(), store, max_upload_bytes);
        let public_url = PublicUrl::local();
        let passkeys = Passkeys::new(&public_url);
        Self {
            pool,
            files,
            public_url,
            passkeys: passkeys
                .unwrap_or_else(|_| unreachable!("localhost is a valid relying party")),
            bootstrap: Bootstrap::default(),
            fetcher: ClientFetcher::default(),
            time_zone: TimeZone::utc(),
            rate_limits: RateLimits::new(),
            backup: Backup::default(),
        }
    }

    pub(crate) fn with_public_url(mut self, public_url: PublicUrl) -> Result<Self, WebauthnError> {
        self.passkeys = Passkeys::new(&public_url)?;
        self.public_url = public_url;
        Ok(self)
    }

    pub(crate) fn with_time_zone(mut self, time_zone: TimeZone) -> Self {
        self.time_zone = time_zone;
        self
    }

    pub(crate) fn with_backup(mut self, backup: Backup) -> Self {
        self.backup = backup;
        self
    }

    pub(crate) fn with_trusted_proxies(mut self, proxies: &[TrustedProxy]) -> Self {
        self.rate_limits = RateLimits::behind(proxies.to_vec());
        self
    }

    pub async fn bootstrap_link(&self) -> Result<Option<String>, sqlx::Error> {
        crate::auth::arm_if_no_users(self).await
    }

    pub(crate) fn pool(&self) -> &PgPool {
        &self.pool
    }

    pub(crate) fn files(&self) -> &Files {
        &self.files
    }

    pub(crate) fn public_url(&self) -> &PublicUrl {
        &self.public_url
    }

    pub(crate) fn time_zone(&self) -> &TimeZone {
        &self.time_zone
    }

    pub(crate) fn backup(&self) -> &Backup {
        &self.backup
    }

    pub(crate) fn rate_limits(&self) -> &RateLimits {
        &self.rate_limits
    }

    pub(crate) fn passkeys(&self) -> &Passkeys {
        &self.passkeys
    }

    pub(crate) fn fetcher(&self) -> &ClientFetcher {
        &self.fetcher
    }

    pub(crate) fn bootstrap(&self) -> &Bootstrap {
        &self.bootstrap
    }
}
