use std::convert::Infallible;
use std::fmt;
use std::net::SocketAddr;
use std::str::FromStr;

use clap::Parser;
use url::{Host, Url};

use crate::backup::BackupConfig;
use crate::rate_limit::TrustedProxy;
use crate::storage::StorageConfig;
use crate::time_zone::TimeZone;

#[derive(Debug, Parser)]
#[command(name = "memory-server", version)]
pub struct Config {
    #[arg(long, env = "DATABASE_URL")]
    database_url: DatabaseUrl,
    #[arg(long, env = "MEMORY_LISTEN", default_value = "0.0.0.0:8080")]
    listen: SocketAddr,
    #[arg(long, env = "MEMORY_MAX_UPLOAD_BYTES", default_value_t = 52_428_800)]
    max_upload_bytes: u64,
    #[command(flatten)]
    storage: StorageConfig,
    #[command(flatten)]
    backup: BackupConfig,
    #[arg(
        long,
        env = "MEMORY_PUBLIC_URL",
        default_value = "http://localhost:8080"
    )]
    public_url: PublicUrl,
    #[arg(long, env = "MEMORY_TIMEZONE", default_value = "UTC")]
    time_zone: TimeZone,
    #[arg(long, env = "MEMORY_TRUSTED_PROXIES", value_delimiter = ',')]
    trusted_proxies: Vec<TrustedProxy>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PublicUrl {
    url: Url,
    issuer: String,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum PublicUrlError {
    #[error("not a valid url")]
    Invalid,
    #[error("scheme must be https, or http for localhost")]
    Scheme,
    #[error("url must be an origin without credentials, path, query or fragment")]
    NotAnOrigin,
}

#[derive(Clone)]
pub(crate) struct DatabaseUrl(String);

impl Config {
    pub(crate) fn database_url(&self) -> &DatabaseUrl {
        &self.database_url
    }

    pub(crate) fn listen(&self) -> SocketAddr {
        self.listen
    }

    pub(crate) fn max_upload_bytes(&self) -> u64 {
        self.max_upload_bytes
    }

    pub(crate) fn storage(&self) -> &StorageConfig {
        &self.storage
    }

    pub(crate) fn backup(&self) -> &BackupConfig {
        &self.backup
    }

    pub(crate) fn public_url(&self) -> &PublicUrl {
        &self.public_url
    }

    pub(crate) fn time_zone(&self) -> &TimeZone {
        &self.time_zone
    }

    pub(crate) fn trusted_proxies(&self) -> &[TrustedProxy] {
        &self.trusted_proxies
    }
}

impl PublicUrl {
    pub(crate) fn local() -> Self {
        Self {
            url: Url::parse("http://localhost:8080").unwrap_or_else(|_| unreachable!()),
            issuer: "http://localhost:8080".to_owned(),
        }
    }

    pub(crate) fn issuer(&self) -> &str {
        &self.issuer
    }

    pub(crate) fn origin(&self) -> &Url {
        &self.url
    }

    pub(crate) fn rp_id(&self) -> &str {
        self.url.host_str().unwrap_or_default()
    }

    pub(crate) fn resource(&self) -> String {
        self.join("/mcp")
    }

    pub(crate) fn join(&self, path: &str) -> String {
        format!("{}{path}", self.issuer)
    }
}

impl FromStr for PublicUrl {
    type Err = PublicUrlError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let url = Url::parse(value).map_err(|_| PublicUrlError::Invalid)?;
        let host = url.host().ok_or(PublicUrlError::Invalid)?;
        let local = match host {
            Host::Domain(domain) => domain == "localhost",
            Host::Ipv4(address) => address.is_loopback(),
            Host::Ipv6(address) => address.is_loopback(),
        };
        match url.scheme() {
            "https" => {}
            "http" if local => {}
            _ => return Err(PublicUrlError::Scheme),
        }
        let clean = url.username().is_empty()
            && url.password().is_none()
            && url.path() == "/"
            && url.query().is_none()
            && url.fragment().is_none();
        if !clean {
            return Err(PublicUrlError::NotAnOrigin);
        }
        let issuer = url.as_str().trim_end_matches('/').to_owned();
        Ok(Self { url, issuer })
    }
}

impl DatabaseUrl {
    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

impl FromStr for DatabaseUrl {
    type Err = Infallible;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        Ok(Self(value.to_owned()))
    }
}

impl fmt::Debug for DatabaseUrl {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[redacted]")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn database_url_is_redacted_in_debug_output() {
        let config = Config::parse_from([
            "memory-server",
            "--database-url",
            "postgres://u:hunter2@h/d",
        ]);
        assert!(!format!("{config:?}").contains("hunter2"));
    }
}
