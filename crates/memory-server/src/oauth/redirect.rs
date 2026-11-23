use url::{Host, Url};

const MAX_LENGTH: usize = 2048;

pub(crate) fn parse_registered(input: &str) -> Option<Url> {
    if input.len() > MAX_LENGTH {
        return None;
    }
    let url = Url::parse(input).ok()?;
    let clean = url.fragment().is_none() && url.username().is_empty() && url.password().is_none();
    let allowed = match url.scheme() {
        "https" => url.host().is_some(),
        "http" => is_loopback(&url),
        _ => false,
    };
    (clean && allowed).then_some(url)
}

pub(crate) fn is_loopback(url: &Url) -> bool {
    match url.host() {
        Some(Host::Domain(domain)) => domain == "localhost",
        Some(Host::Ipv4(address)) => address.is_loopback(),
        Some(Host::Ipv6(address)) => address.is_loopback(),
        None => false,
    }
}

pub(crate) fn matches(registered: &[String], requested: &str) -> bool {
    let Some(requested_url) = parse_registered(requested) else {
        return false;
    };
    registered.iter().any(|candidate| {
        if candidate == requested {
            return true;
        }
        let Some(candidate_url) = parse_registered(candidate) else {
            return false;
        };
        is_loopback(&candidate_url)
            && is_loopback(&requested_url)
            && candidate_url.scheme() == requested_url.scheme()
            && candidate_url.host() == requested_url.host()
            && candidate_url.path() == requested_url.path()
            && candidate_url.query() == requested_url.query()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn list(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn registrations_allow_https_and_loopback_http_only() {
        assert!(parse_registered("https://claude.ai/api/mcp/auth_callback").is_some());
        assert!(parse_registered("http://127.0.0.1:3000/callback").is_some());
        assert!(parse_registered("http://[::1]:3000/callback").is_some());
        assert!(parse_registered("http://localhost/cb").is_some());
        for bad in [
            "http://example.com/cb",
            "cursor://callback",
            "https://example.com/cb#frag",
            "https://user@example.com/cb",
            "javascript:alert(1)",
            "/relative",
        ] {
            assert!(parse_registered(bad).is_none(), "{bad}");
        }
    }

    #[test]
    fn cursors_documented_redirects_are_accepted_and_private_use_schemes_are_not() {
        let registered = list(&[
            "https://www.cursor.com/agents/mcp/oauth/callback",
            "http://localhost:8787/callback",
        ]);
        for uri in &registered {
            assert!(parse_registered(uri).is_some(), "{uri}");
            assert!(matches(&registered, uri), "{uri}");
        }
        assert!(parse_registered("com.example.app:/oauth2redirect").is_none());
        assert!(!matches(
            &list(&["com.example.app:/oauth2redirect"]),
            "com.example.app:/oauth2redirect"
        ));
    }

    #[test]
    fn web_redirects_match_exactly() {
        let registered = list(&["https://app.example/cb"]);
        assert!(matches(&registered, "https://app.example/cb"));
        assert!(!matches(&registered, "https://app.example/cb/"));
        assert!(!matches(&registered, "https://app.example/cb?x=1"));
        assert!(!matches(&registered, "https://app.example:8443/cb"));
        assert!(!matches(&registered, "https://app.example.evil/cb"));
    }

    #[test]
    fn loopback_redirects_ignore_only_the_port() {
        let registered = list(&["http://127.0.0.1:3000/callback"]);
        assert!(matches(&registered, "http://127.0.0.1:51234/callback"));
        assert!(!matches(&registered, "http://127.0.0.1:51234/other"));
        assert!(!matches(&registered, "http://localhost:51234/callback"));
        assert!(!matches(&registered, "https://127.0.0.1:51234/callback"));
        assert!(!matches(&registered, "http://127.0.0.2:51234/callback"));
    }

    #[test]
    fn a_web_registration_never_gains_port_freedom() {
        let registered = list(&["https://app.example/cb"]);
        assert!(!matches(&registered, "https://app.example:444/cb"));
    }
}
