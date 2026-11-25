use std::fmt::Write;

use axum::http::HeaderValue;
use axum::http::header::{
    ACCEPT_RANGES, CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_SECURITY_POLICY, CONTENT_TYPE, ETAG,
    X_CONTENT_TYPE_OPTIONS,
};
use axum::response::Response;

use crate::files::FileRecord;

pub(super) fn with_common_headers(
    mut response: Response,
    record: &FileRecord,
    tag: &HeaderValue,
    download: Option<bool>,
) -> Response {
    let headers = response.headers_mut();
    headers.insert(ETAG, tag.clone());
    headers.insert(ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("private, no-cache"));
    headers.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    headers.insert(
        CONTENT_SECURITY_POLICY,
        HeaderValue::from_static("default-src 'none'; sandbox"),
    );
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_str(&record.mime)
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    if let Some(value) = disposition(record, download.unwrap_or(false)) {
        headers.insert(CONTENT_DISPOSITION, value);
    }
    response
}

fn is_displayable(mime: &str) -> bool {
    mime == "application/pdf"
        || ["image/", "audio/", "text/"]
            .iter()
            .any(|prefix| mime.starts_with(prefix))
}

fn disposition(record: &FileRecord, force_download: bool) -> Option<HeaderValue> {
    let kind = if !force_download && is_displayable(&record.mime) {
        "inline"
    } else {
        "attachment"
    };
    let name = record.path.as_str().rsplit('/').next().unwrap_or_default();
    let fallback: String = name
        .chars()
        .map(|character| {
            if character.is_ascii_graphic() && !matches!(character, '"' | '\\' | '%') {
                character
            } else {
                '_'
            }
        })
        .collect();
    HeaderValue::from_str(&format!(
        "{kind}; filename=\"{fallback}\"; filename*=UTF-8''{}",
        percent_encode(name)
    ))
    .ok()
}

fn percent_encode(text: &str) -> String {
    let mut encoded = String::new();
    for byte in text.bytes() {
        if byte.is_ascii_alphanumeric() || b"!#$&+-.^_`|~".contains(&byte) {
            encoded.push(char::from(byte));
        } else {
            let _ = write!(encoded, "%{byte:02X}");
        }
    }
    encoded
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_ascii_names_get_an_encoded_filename() {
        assert_eq!(
            percent_encode("계약서 1.pdf"),
            "%EA%B3%84%EC%95%BD%EC%84%9C%201.pdf"
        );
    }
}
