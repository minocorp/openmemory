#[cfg(test)]
mod tests;
mod trusted_proxy;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::num::NonZeroU32;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use axum::Json;
use axum::extract::{ConnectInfo, Request, State};
use axum::http::header::{AUTHORIZATION, RETRY_AFTER};
use axum::http::{HeaderValue, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use governor::clock::{Clock, DefaultClock};
use governor::{DefaultKeyedRateLimiter, Quota};
use serde_json::json;

pub(crate) use trusted_proxy::TrustedProxy;

const FORWARDED_FOR: &str = "x-forwarded-for";
const IPV6_PREFIX_BYTES: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Policy {
    Auth,
    Token,
    Registration,
    Authorize,
    UploadLink,
    Bearer,
}

#[derive(Clone)]
pub(crate) struct RateLimits {
    auth: Arc<DefaultKeyedRateLimiter<IpAddr>>,
    token: Arc<DefaultKeyedRateLimiter<IpAddr>>,
    registration: Arc<DefaultKeyedRateLimiter<IpAddr>>,
    authorize: Arc<DefaultKeyedRateLimiter<IpAddr>>,
    upload_link: Arc<DefaultKeyedRateLimiter<IpAddr>>,
    bearer: Arc<DefaultKeyedRateLimiter<IpAddr>>,
    proxies: Arc<[TrustedProxy]>,
    warned_unconfigured_proxy: Arc<AtomicBool>,
}

impl Policy {
    fn of(path: &str, has_authorization: bool) -> Option<Self> {
        match path {
            "/oauth/token" => Some(Self::Token),
            "/oauth/register" | "/oauth/device" => Some(Self::Registration),
            "/oauth/authorize" | "/oauth/revoke" => Some(Self::Authorize),
            _ if path.starts_with("/api/upload-links/") => Some(Self::UploadLink),
            _ if path.starts_with("/api/auth/") => Some(Self::Auth),
            _ if has_authorization && path.starts_with("/api/") => Some(Self::Bearer),
            _ => None,
        }
    }
}

impl RateLimits {
    pub(crate) fn new() -> Self {
        Self::behind(Vec::new())
    }

    pub(crate) fn behind(proxies: Vec<TrustedProxy>) -> Self {
        Self {
            proxies: proxies.into(),
            auth: limiter(30, Duration::from_secs(2)),
            token: limiter(60, Duration::from_secs(1)),
            registration: limiter(10, Duration::from_secs(6)),
            authorize: limiter(30, Duration::from_secs(2)),
            upload_link: limiter(20, Duration::from_secs(3)),
            bearer: limiter(120, Duration::from_millis(500)),
            warned_unconfigured_proxy: Arc::new(AtomicBool::new(false)),
        }
    }

    pub(crate) fn prune(&self) {
        for limiter in [
            &self.auth,
            &self.token,
            &self.registration,
            &self.authorize,
            &self.upload_link,
            &self.bearer,
        ] {
            limiter.retain_recent();
            limiter.shrink_to_fit();
        }
    }

    fn limiter(&self, policy: Policy) -> &DefaultKeyedRateLimiter<IpAddr> {
        match policy {
            Policy::Auth => &self.auth,
            Policy::Token => &self.token,
            Policy::Registration => &self.registration,
            Policy::Authorize => &self.authorize,
            Policy::UploadLink => &self.upload_link,
            Policy::Bearer => &self.bearer,
        }
    }
}

impl std::fmt::Debug for RateLimits {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("RateLimits")
    }
}

fn is_internal(address: IpAddr) -> bool {
    match address.to_canonical() {
        IpAddr::V4(v4) => v4.is_loopback() || v4.is_private() || v4.is_link_local(),
        IpAddr::V6(v6) => v6.is_loopback() || v6.is_unique_local() || v6.is_unicast_link_local(),
    }
}

fn limiter(burst: u32, replenish_every: Duration) -> Arc<DefaultKeyedRateLimiter<IpAddr>> {
    let burst = NonZeroU32::new(burst).unwrap_or(NonZeroU32::MIN);
    let quota = Quota::with_period(replenish_every)
        .unwrap_or_else(|| Quota::per_second(NonZeroU32::MIN))
        .allow_burst(burst);
    Arc::new(DefaultKeyedRateLimiter::keyed(quota))
}

pub(crate) async fn enforce(
    State(limits): State<RateLimits>,
    request: Request,
    next: Next,
) -> Response {
    let has_authorization = request.headers().contains_key(AUTHORIZATION);
    let Some(policy) = Policy::of(request.uri().path(), has_authorization) else {
        return next.run(request).await;
    };
    match limits
        .limiter(policy)
        .check_key(&limits.client_key(&request))
    {
        Ok(()) => next.run(request).await,
        Err(not_until) => {
            let wait = not_until.wait_time_from(DefaultClock::default().now());
            too_many_requests(wait)
        }
    }
}

fn too_many_requests(wait: Duration) -> Response {
    let seconds = wait.as_secs().saturating_add(1);
    let mut response = (
        StatusCode::TOO_MANY_REQUESTS,
        Json(json!({ "error": "rate_limited" })),
    )
        .into_response();
    if let Ok(value) = HeaderValue::from_str(&seconds.to_string()) {
        response.headers_mut().insert(RETRY_AFTER, value);
    }
    response
}

impl RateLimits {
    fn client_key(&self, request: &Request) -> IpAddr {
        let peer = request
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .map_or(IpAddr::V4(Ipv4Addr::UNSPECIFIED), |info| info.0.ip());
        self.warn_if_proxy_unconfigured(request, peer);
        let address = self.forwarded_client(request, peer).unwrap_or(peer);
        match address.to_canonical() {
            IpAddr::V4(v4) => IpAddr::V4(v4),
            IpAddr::V6(v6) => {
                let mut octets = [0_u8; 16];
                octets
                    .iter_mut()
                    .zip(v6.octets())
                    .take(IPV6_PREFIX_BYTES)
                    .for_each(|(slot, byte)| *slot = byte);
                IpAddr::V6(octets.into())
            }
        }
    }

    fn warn_if_proxy_unconfigured(&self, request: &Request, peer: IpAddr) {
        if self.proxies.is_empty()
            && is_internal(peer)
            && request.headers().contains_key(FORWARDED_FOR)
            && !self.warned_unconfigured_proxy.swap(true, Ordering::Relaxed)
        {
            tracing::warn!(
                %peer,
                "X-Forwarded-For arrived from an internal address but MEMORY_TRUSTED_PROXIES is empty: every client shares one rate limit bucket"
            );
        }
    }

    fn is_trusted(&self, address: IpAddr) -> bool {
        self.proxies.iter().any(|proxy| proxy.contains(address))
    }

    // Walk X-Forwarded-For from the right past trusted proxies; the first other entry is the client.
    fn forwarded_client(&self, request: &Request, peer: IpAddr) -> Option<IpAddr> {
        if !self.is_trusted(peer) {
            return None;
        }
        let entries: Vec<&str> = request
            .headers()
            .get_all(FORWARDED_FOR)
            .iter()
            .filter_map(|value| value.to_str().ok())
            .flat_map(|value| value.split(','))
            .collect();
        for entry in entries.into_iter().rev() {
            let address = entry.trim().parse::<IpAddr>().ok()?;
            if !self.is_trusted(address) {
                return Some(address);
            }
        }
        None
    }
}
