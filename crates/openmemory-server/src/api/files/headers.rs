use std::fmt::Write;

use axum::http::header::{
    ACCEPT_RANGES, CACHE_CONTROL, CONTENT_DISPOSITION, CONTENT_SECURITY_POLICY, CONTENT_TYPE, ETAG,
    VARY, X_CONTENT_TYPE_OPTIONS, X_FRAME_OPTIONS,
};
use axum::http::{HeaderMap, HeaderValue};
use axum::response::Response;

use crate::files::FileRecord;

const FILE_POLICY: &str = "default-src 'none'; sandbox";
const SVG_POLICY: &str =
    "default-src 'none'; style-src 'unsafe-inline'; img-src data:; font-src data:; sandbox";
const PAGE_POLICY: &str = "sandbox allow-scripts; default-src 'none'; script-src 'unsafe-inline' 'unsafe-eval' https:; style-src 'unsafe-inline' https:; img-src data: blob: https:; font-src data: https:; media-src data: blob: https:; connect-src 'none'; form-action 'none'; base-uri 'none'; frame-ancestors 'self'";
const SOURCE_TYPE: &str = "text/plain; charset=utf-8";
const SEC_FETCH_DEST: &str = "sec-fetch-dest";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Rendering {
    Page,
    Svg,
    Source,
    Native,
}

impl Rendering {
    fn of(mime: &str) -> Self {
        match mime {
            "text/html" => Self::Page,
            "image/svg+xml" => Self::Svg,
            "application/xml" | "text/xml" => Self::Source,
            _ => Self::Native,
        }
    }

    fn policy(self) -> &'static str {
        match self {
            Self::Page => PAGE_POLICY,
            Self::Svg => SVG_POLICY,
            Self::Source | Self::Native => FILE_POLICY,
        }
    }

    fn shows_inline(self, mime: &str, request: &HeaderMap) -> bool {
        match self {
            Self::Page => request
                .get(SEC_FETCH_DEST)
                .is_some_and(|value| value.as_bytes() == b"iframe"),
            Self::Svg | Self::Source => true,
            Self::Native => {
                mime == "application/pdf"
                    || ["image/", "audio/", "text/"]
                        .iter()
                        .any(|prefix| mime.starts_with(prefix))
            }
        }
    }
}

pub(super) fn with_content_headers(
    mut response: Response,
    record: &FileRecord,
    tag: &HeaderValue,
    request: &HeaderMap,
    download: Option<bool>,
) -> Response {
    let rendering = Rendering::of(&record.mime);
    let inline = !download.unwrap_or(false) && rendering.shows_inline(&record.mime, request);
    let headers = response.headers_mut();
    headers.insert(ETAG, tag.clone());
    headers.insert(ACCEPT_RANGES, HeaderValue::from_static("bytes"));
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("private, no-cache"));
    headers.insert(X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff"));
    headers.insert(
        CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(rendering.policy()),
    );
    headers.insert(CONTENT_TYPE, content_type(rendering, &record.mime));
    if rendering == Rendering::Page {
        headers.insert(VARY, HeaderValue::from_static("Sec-Fetch-Dest"));
        if inline {
            headers.insert(X_FRAME_OPTIONS, HeaderValue::from_static("SAMEORIGIN"));
        }
    }
    if let Some(value) = disposition(record, inline) {
        headers.insert(CONTENT_DISPOSITION, value);
    }
    response
}

fn content_type(rendering: Rendering, mime: &str) -> HeaderValue {
    if rendering == Rendering::Source {
        return HeaderValue::from_static(SOURCE_TYPE);
    }
    HeaderValue::from_str(mime)
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"))
}

fn disposition(record: &FileRecord, inline: bool) -> Option<HeaderValue> {
    let kind = if inline { "inline" } else { "attachment" };
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

    #[test]
    fn html_shows_inline_only_inside_a_frame() {
        let mut framed = HeaderMap::new();
        framed.insert(SEC_FETCH_DEST, HeaderValue::from_static("iframe"));
        let mut opened = HeaderMap::new();
        opened.insert(SEC_FETCH_DEST, HeaderValue::from_static("document"));
        assert!(Rendering::Page.shows_inline("text/html", &framed));
        assert!(!Rendering::Page.shows_inline("text/html", &opened));
        assert!(!Rendering::Page.shows_inline("text/html", &HeaderMap::new()));
    }

    #[test]
    fn markup_types_get_their_own_rendering() {
        assert_eq!(Rendering::of("text/html"), Rendering::Page);
        assert_eq!(Rendering::of("image/svg+xml"), Rendering::Svg);
        assert_eq!(Rendering::of("text/xml"), Rendering::Source);
        assert_eq!(Rendering::of("application/xml"), Rendering::Source);
        assert_eq!(Rendering::of("text/plain"), Rendering::Native);
        assert!(!Rendering::Svg.policy().contains("allow-scripts"));
        assert!(Rendering::Page.policy().contains("connect-src 'none'"));
    }
}
