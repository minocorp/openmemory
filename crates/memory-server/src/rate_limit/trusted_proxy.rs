use std::net::IpAddr;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TrustedProxy {
    network: IpAddr,
    prefix: u8,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum TrustedProxyError {
    #[error("not an ip address or cidr range")]
    Invalid,
    #[error("prefix length is too long for the address family")]
    PrefixTooLong,
}

impl TrustedProxy {
    pub(crate) fn contains(&self, address: IpAddr) -> bool {
        match (self.network, address.to_canonical()) {
            (IpAddr::V4(network), IpAddr::V4(address)) => {
                let mask = u32::MAX
                    .checked_shl(32 - u32::from(self.prefix))
                    .unwrap_or(0);
                u32::from(network) & mask == u32::from(address) & mask
            }
            (IpAddr::V6(network), IpAddr::V6(address)) => {
                let mask = u128::MAX
                    .checked_shl(128 - u32::from(self.prefix))
                    .unwrap_or(0);
                u128::from(network) & mask == u128::from(address) & mask
            }
            (IpAddr::V4(_), IpAddr::V6(_)) | (IpAddr::V6(_), IpAddr::V4(_)) => false,
        }
    }
}

impl FromStr for TrustedProxy {
    type Err = TrustedProxyError;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (address, prefix) = match value.split_once('/') {
            Some((address, prefix)) => (address, Some(prefix)),
            None => (value, None),
        };
        let network = address
            .parse::<IpAddr>()
            .map_err(|_| TrustedProxyError::Invalid)?
            .to_canonical();
        let longest: u8 = if network.is_ipv4() { 32 } else { 128 };
        let prefix = match prefix {
            Some(prefix) => prefix.parse().map_err(|_| TrustedProxyError::Invalid)?,
            None => longest,
        };
        if prefix > longest {
            return Err(TrustedProxyError::PrefixTooLong);
        }
        Ok(Self { network, prefix })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn proxy(value: &str) -> TrustedProxy {
        value.parse().unwrap()
    }

    fn ip(value: &str) -> IpAddr {
        value.parse().unwrap()
    }

    #[test]
    fn a_bare_address_matches_only_itself() {
        assert!(proxy("172.18.0.2").contains(ip("172.18.0.2")));
        assert!(!proxy("172.18.0.2").contains(ip("172.18.0.3")));
    }

    #[test]
    fn a_cidr_range_matches_its_members() {
        assert!(proxy("172.16.0.0/12").contains(ip("172.31.255.255")));
        assert!(!proxy("172.16.0.0/12").contains(ip("172.32.0.1")));
        assert!(proxy("fd00::/8").contains(ip("fd12::1")));
        assert!(!proxy("fd00::/8").contains(ip("fe80::1")));
    }

    #[test]
    fn a_zero_prefix_matches_every_address_of_its_family() {
        assert!(proxy("0.0.0.0/0").contains(ip("203.0.113.9")));
        assert!(!proxy("0.0.0.0/0").contains(ip("2001:db8::1")));
    }

    #[test]
    fn mapped_ipv4_peers_match_ipv4_ranges() {
        assert!(proxy("10.0.0.0/8").contains(ip("::ffff:10.1.2.3")));
    }

    #[test]
    fn malformed_values_are_rejected() {
        assert_eq!(
            "nope".parse::<TrustedProxy>(),
            Err(TrustedProxyError::Invalid)
        );
        assert_eq!(
            "10.0.0.0/x".parse::<TrustedProxy>(),
            Err(TrustedProxyError::Invalid)
        );
        assert_eq!(
            "10.0.0.0/33".parse::<TrustedProxy>(),
            Err(TrustedProxyError::PrefixTooLong)
        );
    }
}
