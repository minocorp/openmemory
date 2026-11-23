use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use tokio::net::lookup_host;
use tokio::sync::Semaphore;
use url::{Host, Url};

const MAX_BODY_BYTES: usize = 10 * 1024;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);
const RESOLVE_TIMEOUT: Duration = Duration::from_secs(3);
const MAX_CONCURRENT_FETCHES: usize = 8;
const FAILURE_TTL: Duration = Duration::from_secs(60);
const MAX_CACHED_FAILURES: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Reach {
    PublicOnly,
    #[cfg(test)]
    Anywhere,
}

#[derive(Debug, Clone)]
pub(crate) struct ClientFetcher {
    permits: Arc<Semaphore>,
    reach: Reach,
    failures: Arc<Mutex<HashMap<String, (Instant, FetchError)>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub(crate) enum FetchError {
    #[error("address is not public")]
    Blocked,
    #[error("host does not resolve")]
    Unresolved,
    #[error("too many fetches in flight")]
    Busy,
    #[error("request failed")]
    Request,
    #[error("response status is not 200")]
    Status,
    #[error("response is too large")]
    TooLarge,
}

impl Default for ClientFetcher {
    fn default() -> Self {
        Self {
            permits: Arc::new(Semaphore::new(MAX_CONCURRENT_FETCHES)),
            reach: Reach::PublicOnly,
            failures: Arc::default(),
        }
    }
}

impl ClientFetcher {
    #[cfg(test)]
    fn anywhere() -> Self {
        Self {
            reach: Reach::Anywhere,
            ..Self::default()
        }
    }

    pub(crate) async fn fetch_json(&self, url: &Url) -> Result<Vec<u8>, FetchError> {
        if let Some(error) = self.recent_failure(url) {
            return Err(error);
        }
        let result = self.fetch(url).await;
        if let Err(error) = result
            && error != FetchError::Busy
        {
            self.remember_failure(url, error);
        }
        result
    }

    fn recent_failure(&self, url: &Url) -> Option<FetchError> {
        let failures = self.failures.lock().unwrap_or_else(PoisonError::into_inner);
        failures
            .get(url.as_str())
            .filter(|(at, _)| at.elapsed() < FAILURE_TTL)
            .map(|(_, error)| *error)
    }

    fn remember_failure(&self, url: &Url, error: FetchError) {
        let mut failures = self.failures.lock().unwrap_or_else(PoisonError::into_inner);
        failures.retain(|_, (at, _)| at.elapsed() < FAILURE_TTL);
        if failures.len() < MAX_CACHED_FAILURES {
            failures.insert(url.as_str().to_owned(), (Instant::now(), error));
        }
    }

    async fn fetch(&self, url: &Url) -> Result<Vec<u8>, FetchError> {
        let _permit = self.permits.try_acquire().map_err(|_| FetchError::Busy)?;
        let host = url.host().ok_or(FetchError::Unresolved)?;
        let port = url.port_or_known_default().ok_or(FetchError::Unresolved)?;
        let addresses = self.resolve(&host, port).await?;
        let mut builder = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(REQUEST_TIMEOUT)
            .connect_timeout(CONNECT_TIMEOUT)
            .no_proxy()
            .user_agent("memory-server");
        if let Host::Domain(domain) = &host {
            builder = builder.resolve_to_addrs(domain, &addresses);
        }
        let client = builder.build().map_err(|_| FetchError::Request)?;
        let mut response = client
            .get(url.as_str())
            .header("accept", "application/json")
            .send()
            .await
            .map_err(|_| FetchError::Request)?;
        if response.status() != reqwest::StatusCode::OK {
            return Err(FetchError::Status);
        }
        let mut body = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|_| FetchError::Request)? {
            if body.len() + chunk.len() > MAX_BODY_BYTES {
                return Err(FetchError::TooLarge);
            }
            body.extend_from_slice(&chunk);
        }
        Ok(body)
    }

    async fn resolve(&self, host: &Host<&str>, port: u16) -> Result<Vec<SocketAddr>, FetchError> {
        let addresses: Vec<SocketAddr> = match host {
            Host::Ipv4(address) => vec![SocketAddr::new(IpAddr::V4(*address), port)],
            Host::Ipv6(address) => vec![SocketAddr::new(IpAddr::V6(*address), port)],
            Host::Domain(domain) => {
                tokio::time::timeout(RESOLVE_TIMEOUT, lookup_host((*domain, port)))
                    .await
                    .map_err(|_| FetchError::Unresolved)?
                    .map_err(|_| FetchError::Unresolved)?
                    .collect()
            }
        };
        if addresses.is_empty() {
            return Err(FetchError::Unresolved);
        }
        if self.reach == Reach::PublicOnly && addresses.iter().any(|a| !is_public(a.ip())) {
            return Err(FetchError::Blocked);
        }
        Ok(addresses)
    }
}

pub(crate) fn is_public(address: IpAddr) -> bool {
    match address {
        IpAddr::V4(address) => is_public_v4(address),
        IpAddr::V6(address) => match address.to_ipv4_mapped() {
            Some(mapped) => is_public_v4(mapped),
            None => is_public_v6(address),
        },
    }
}

fn is_public_v4(address: Ipv4Addr) -> bool {
    let [a, b, c, _] = address.octets();
    let special = address.is_unspecified()
        || address.is_loopback()
        || address.is_private()
        || address.is_link_local()
        || address.is_broadcast()
        || address.is_multicast()
        || address.is_documentation()
        || a == 0
        || (a == 100 && (64..=127).contains(&b))
        || (a == 192 && b == 0 && c == 0)
        || (a == 198 && (b == 18 || b == 19))
        || a >= 240;
    !special
}

fn is_public_v6(address: Ipv6Addr) -> bool {
    let first = address.segments()[0];
    let second = address.segments()[1];
    let global_unicast = (first & 0xe000) == 0x2000;
    let documentation = first == 0x2001 && second == 0x0db8;
    let teredo = first == 0x2001 && second == 0;
    let six_to_four = first == 0x2002;
    global_unicast && !documentation && !teredo && !six_to_four
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    async fn serve(response: &'static str) -> Url {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            if let Ok((mut socket, _)) = listener.accept().await {
                let mut buffer = [0_u8; 1024];
                let _ = socket.read(&mut buffer).await;
                let _ = socket.write_all(response.as_bytes()).await;
            }
        });
        Url::parse(&format!("http://127.0.0.1:{port}/client.json")).unwrap()
    }

    #[test]
    fn non_public_addresses_are_blocked() {
        let blocked = [
            "127.0.0.1",
            "10.1.2.3",
            "172.16.0.1",
            "192.168.1.1",
            "169.254.169.254",
            "100.64.0.1",
            "0.0.0.0",
            "224.0.0.1",
            "255.255.255.255",
            "198.18.0.1",
            "192.0.2.1",
            "::1",
            "::",
            "fe80::1",
            "fc00::1",
            "fd12:3456::1",
            "ff02::1",
            "::ffff:127.0.0.1",
            "::ffff:10.0.0.1",
            "64:ff9b::7f00:1",
            "2002:7f00:1::1",
            "2001:db8::1",
        ];
        for text in blocked {
            let address: IpAddr = text.parse().unwrap();
            assert!(!is_public(address), "{text}");
        }
    }

    #[test]
    fn public_addresses_are_allowed() {
        for text in ["93.184.216.34", "8.8.8.8", "2606:4700:4700::1111"] {
            let address: IpAddr = text.parse().unwrap();
            assert!(is_public(address), "{text}");
        }
    }

    #[tokio::test]
    async fn the_public_fetcher_refuses_loopback_hosts_without_connecting() {
        let url = serve("HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\n{}").await;
        let result = ClientFetcher::default().fetch_json(&url).await;
        assert_eq!(result, Err(FetchError::Blocked));
    }

    #[tokio::test]
    async fn localhost_by_name_is_blocked() {
        let url = Url::parse("https://localhost/client.json").unwrap();
        let result = ClientFetcher::default().fetch_json(&url).await;
        assert_eq!(result, Err(FetchError::Blocked));
    }

    #[tokio::test]
    async fn redirects_are_not_followed() {
        let url = serve(
            "HTTP/1.1 302 Found\r\nlocation: http://169.254.169.254/\r\ncontent-length: 0\r\n\r\n",
        )
        .await;
        let result = ClientFetcher::anywhere().fetch_json(&url).await;
        assert_eq!(result, Err(FetchError::Status));
    }

    #[tokio::test]
    async fn a_200_document_is_returned() {
        let url = serve("HTTP/1.1 200 OK\r\ncontent-length: 2\r\n\r\n{}").await;
        let result = ClientFetcher::anywhere().fetch_json(&url).await;
        assert_eq!(result, Ok(b"{}".to_vec()));
    }

    #[tokio::test]
    async fn oversized_documents_are_refused() {
        let url = serve(
            "HTTP/1.1 200 OK\r\ncontent-length: 20000\r\n\r\naaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        )
        .await;
        let result = ClientFetcher::anywhere().fetch_json(&url).await;
        assert!(matches!(
            result,
            Err(FetchError::TooLarge | FetchError::Request)
        ));
    }

    #[tokio::test]
    async fn a_silent_server_times_out() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move {
            let _held = listener.accept().await;
            tokio::time::sleep(Duration::from_secs(30)).await;
        });
        let url = Url::parse(&format!("http://127.0.0.1:{port}/c.json")).unwrap();
        let result = ClientFetcher::anywhere().fetch_json(&url).await;
        assert_eq!(result, Err(FetchError::Request));
    }

    #[tokio::test]
    async fn a_failed_fetch_is_remembered_briefly() {
        let url = serve("HTTP/1.1 500 Internal Server Error\r\ncontent-length: 0\r\n\r\n").await;
        let fetcher = ClientFetcher::anywhere();
        assert_eq!(fetcher.fetch_json(&url).await, Err(FetchError::Status));
        assert_eq!(fetcher.fetch_json(&url).await, Err(FetchError::Status));
    }
}
