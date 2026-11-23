use anyhow::{Context, bail};
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Server {
    origin: String,
}

impl Server {
    pub(crate) fn parse(input: &str) -> anyhow::Result<Self> {
        let url = Url::parse(input.trim()).context("server address is not a valid url")?;
        if !matches!(url.scheme(), "http" | "https") || url.host().is_none() {
            bail!("server address must start with https:// or http://");
        }
        Ok(Self {
            origin: url.origin().ascii_serialization(),
        })
    }

    pub(crate) fn from_origin(origin: String) -> Self {
        Self { origin }
    }

    pub(crate) fn origin(&self) -> &str {
        &self.origin
    }

    pub(crate) fn mcp_url(&self) -> String {
        format!("{}/mcp", self.origin)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn any_path_on_the_server_resolves_to_its_mcp_endpoint() {
        let cases = [
            (
                "https://memory.example.com",
                "https://memory.example.com/mcp",
            ),
            (
                "https://memory.example.com/",
                "https://memory.example.com/mcp",
            ),
            (
                "https://memory.example.com/mcp",
                "https://memory.example.com/mcp",
            ),
            ("http://127.0.0.1:8080/a/b?c=d", "http://127.0.0.1:8080/mcp"),
        ];
        for (input, expected) in cases {
            assert_eq!(Server::parse(input).unwrap().mcp_url(), expected, "{input}");
        }
    }

    #[test]
    fn only_http_addresses_are_accepted() {
        for input in [
            "memory.example.com",
            "ftp://memory.example.com",
            "file:///x",
            "",
        ] {
            assert!(Server::parse(input).is_err(), "{input}");
        }
    }
}
