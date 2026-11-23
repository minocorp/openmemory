use serde_json::Value;
use url::Url;

use crate::oauth::redirect;

const MAX_URL_LENGTH: usize = 2048;
const MAX_NAME_CHARS: usize = 128;
const MAX_REDIRECT_URIS: usize = 10;
const GRANT_TYPES: [&str; 3] = [
    "authorization_code",
    "refresh_token",
    "urn:ietf:params:oauth:grant-type:device_code",
];

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Metadata {
    pub(crate) name: String,
    pub(crate) redirect_uris: Vec<String>,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum MetadataError {
    #[error("document is not a json object")]
    NotAnObject,
    #[error("client_id does not match the document url")]
    ClientIdMismatch,
    #[error("client_name is missing or invalid")]
    Name,
    #[error("redirect_uris are missing or invalid")]
    RedirectUris,
    #[error("shared secrets are not supported")]
    SharedSecret,
    #[error("no supported grant or response type")]
    Unsupported,
}

pub(crate) fn parse_client_id(input: &str, own_origin: &Url) -> Option<Url> {
    if input.len() > MAX_URL_LENGTH {
        return None;
    }
    let url = Url::parse(input).ok()?;
    let plain = url.scheme() == "https"
        && url.username().is_empty()
        && url.password().is_none()
        && url.fragment().is_none()
        && url.path() != "/"
        && url.as_str() == input
        && !url
            .path_segments()
            .is_some_and(|mut segments| segments.any(|s| s == "." || s == ".."));
    (plain && url.origin() != own_origin.origin()).then_some(url)
}

pub(crate) fn parse_document(client_id: &str, bytes: &[u8]) -> Result<Metadata, MetadataError> {
    let document: Value = serde_json::from_slice(bytes).map_err(|_| MetadataError::NotAnObject)?;
    let object = document.as_object().ok_or(MetadataError::NotAnObject)?;
    if object.get("client_id").and_then(Value::as_str) != Some(client_id) {
        return Err(MetadataError::ClientIdMismatch);
    }
    reject_shared_secrets(object)?;
    require_overlap(object.get("grant_types"), &GRANT_TYPES)?;
    require_overlap(object.get("response_types"), &["code"])?;
    let name = object
        .get("client_name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| {
            !name.is_empty()
                && name.chars().count() <= MAX_NAME_CHARS
                && !name.chars().any(char::is_control)
        })
        .ok_or(MetadataError::Name)?;
    Ok(Metadata {
        name: name.to_owned(),
        redirect_uris: redirect_uris(object.get("redirect_uris"))?,
    })
}

fn reject_shared_secrets(object: &serde_json::Map<String, Value>) -> Result<(), MetadataError> {
    let secret_keys = ["client_secret", "client_secret_expires_at"];
    if secret_keys.iter().any(|key| object.contains_key(*key)) {
        return Err(MetadataError::SharedSecret);
    }
    if let Some(method) = object.get("token_endpoint_auth_method")
        && method.as_str() != Some("none")
    {
        return Err(MetadataError::SharedSecret);
    }
    if let Some(methods) = object.get("token_endpoint_auth_methods_supported") {
        let allows_none = methods
            .as_array()
            .is_some_and(|list| list.iter().any(|m| m.as_str() == Some("none")));
        if !allows_none {
            return Err(MetadataError::SharedSecret);
        }
    }
    Ok(())
}

fn require_overlap(declared: Option<&Value>, supported: &[&str]) -> Result<(), MetadataError> {
    let Some(declared) = declared else {
        return Ok(());
    };
    let overlaps = declared.as_array().is_some_and(|list| {
        list.iter()
            .filter_map(Value::as_str)
            .any(|v| supported.contains(&v))
    });
    if overlaps {
        Ok(())
    } else {
        Err(MetadataError::Unsupported)
    }
}

fn redirect_uris(value: Option<&Value>) -> Result<Vec<String>, MetadataError> {
    let list = value
        .and_then(Value::as_array)
        .filter(|list| !list.is_empty() && list.len() <= MAX_REDIRECT_URIS)
        .ok_or(MetadataError::RedirectUris)?;
    list.iter()
        .map(|item| {
            item.as_str()
                .filter(|uri| redirect::parse_registered(uri).is_some())
                .map(str::to_owned)
                .ok_or(MetadataError::RedirectUris)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLIENT_ID: &str = "https://app.example/oauth/client.json";

    fn own() -> Url {
        Url::parse("https://memory.example").unwrap()
    }

    fn document(extra: &str) -> Vec<u8> {
        format!(
            r#"{{"client_id":"{CLIENT_ID}","client_name":"Example","redirect_uris":["http://127.0.0.1:3000/cb"]{extra}}}"#
        )
        .into_bytes()
    }

    #[test]
    fn client_id_urls_follow_the_draft_syntax() {
        assert!(parse_client_id(CLIENT_ID, &own()).is_some());
        for bad in [
            "http://app.example/c.json",
            "https://app.example/",
            "https://app.example",
            "https://u:p@app.example/c.json",
            "https://app.example/c.json#x",
            "https://app.example/a/../c.json",
            "https://app.example/a/./c.json",
            "https://memory.example/client.json",
            "not a url",
            "claude-code",
        ] {
            assert!(parse_client_id(bad, &own()).is_none(), "{bad}");
        }
    }

    #[test]
    fn a_valid_document_yields_name_and_redirects() {
        let metadata = parse_document(CLIENT_ID, &document("")).unwrap();
        assert_eq!(metadata.name, "Example");
        assert_eq!(metadata.redirect_uris, vec!["http://127.0.0.1:3000/cb"]);
    }

    #[test]
    fn a_document_for_another_client_id_is_rejected() {
        let result = parse_document("https://other.example/c.json", &document(""));
        assert_eq!(result, Err(MetadataError::ClientIdMismatch));
    }

    #[test]
    fn shared_secret_methods_are_rejected() {
        for extra in [
            r#","client_secret":"x""#,
            r#","token_endpoint_auth_method":"client_secret_basic""#,
            r#","token_endpoint_auth_methods_supported":["client_secret_post"]"#,
        ] {
            assert_eq!(
                parse_document(CLIENT_ID, &document(extra)),
                Err(MetadataError::SharedSecret),
                "{extra}"
            );
        }
        assert!(
            parse_document(
                CLIENT_ID,
                &document(r#","token_endpoint_auth_method":"none""#)
            )
            .is_ok()
        );
    }

    #[test]
    fn unsupported_grants_and_bad_redirects_are_rejected() {
        assert_eq!(
            parse_document(CLIENT_ID, &document(r#","grant_types":["implicit"]"#)),
            Err(MetadataError::Unsupported)
        );
        assert_eq!(
            parse_document(CLIENT_ID, &document(r#","response_types":["token"]"#)),
            Err(MetadataError::Unsupported)
        );
        let bad = format!(
            r#"{{"client_id":"{CLIENT_ID}","client_name":"x","redirect_uris":["http://evil.example/cb"]}}"#
        );
        assert_eq!(
            parse_document(CLIENT_ID, bad.as_bytes()),
            Err(MetadataError::RedirectUris)
        );
        assert_eq!(
            parse_document(CLIENT_ID, b"[]"),
            Err(MetadataError::NotAnObject)
        );
    }

    #[test]
    fn the_name_is_required_and_clean() {
        let nameless =
            format!(r#"{{"client_id":"{CLIENT_ID}","redirect_uris":["https://app.example/cb"]}}"#);
        assert_eq!(
            parse_document(CLIENT_ID, nameless.as_bytes()),
            Err(MetadataError::Name)
        );
    }
}
