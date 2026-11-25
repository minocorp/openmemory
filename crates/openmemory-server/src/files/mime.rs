use openmemory_ops::PagePath;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileKind {
    Pdf,
    Text,
    Image,
    Spreadsheet,
    Audio,
    Document,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AllowedMime {
    essence: &'static str,
    kind: FileKind,
}

const UNTYPED: &str = "application/octet-stream";

const ALLOWED: &[(&str, FileKind)] = &[
    ("application/pdf", FileKind::Pdf),
    ("text/plain", FileKind::Text),
    ("text/markdown", FileKind::Text),
    ("text/csv", FileKind::Text),
    ("text/html", FileKind::Text),
    ("application/xml", FileKind::Text),
    ("text/xml", FileKind::Text),
    ("image/png", FileKind::Image),
    ("image/jpeg", FileKind::Image),
    ("image/gif", FileKind::Image),
    ("image/webp", FileKind::Image),
    ("image/heic", FileKind::Image),
    ("image/avif", FileKind::Image),
    ("image/svg+xml", FileKind::Image),
    ("application/vnd.ms-excel", FileKind::Spreadsheet),
    (
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        FileKind::Spreadsheet,
    ),
    (
        "application/vnd.oasis.opendocument.spreadsheet",
        FileKind::Spreadsheet,
    ),
    ("audio/mpeg", FileKind::Audio),
    ("audio/mp4", FileKind::Audio),
    ("audio/x-m4a", FileKind::Audio),
    ("audio/aac", FileKind::Audio),
    ("audio/wav", FileKind::Audio),
    ("audio/x-wav", FileKind::Audio),
    ("audio/ogg", FileKind::Audio),
    ("audio/flac", FileKind::Audio),
    ("audio/webm", FileKind::Audio),
    (
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        FileKind::Document,
    ),
];

const BY_EXTENSION: &[(&str, &str)] = &[
    ("pdf", "application/pdf"),
    ("txt", "text/plain"),
    ("md", "text/markdown"),
    ("csv", "text/csv"),
    ("html", "text/html"),
    ("htm", "text/html"),
    ("xml", "application/xml"),
    ("png", "image/png"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("gif", "image/gif"),
    ("webp", "image/webp"),
    ("heic", "image/heic"),
    ("avif", "image/avif"),
    ("svg", "image/svg+xml"),
    ("xls", "application/vnd.ms-excel"),
    (
        "xlsx",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
    ),
    ("ods", "application/vnd.oasis.opendocument.spreadsheet"),
    ("mp3", "audio/mpeg"),
    ("m4a", "audio/mp4"),
    ("aac", "audio/aac"),
    ("wav", "audio/wav"),
    ("ogg", "audio/ogg"),
    ("flac", "audio/flac"),
    ("webm", "audio/webm"),
    (
        "docx",
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    ),
];

pub(crate) fn mime_for_path(path: &PagePath) -> Option<&'static str> {
    let (_, extension) = path.as_str().rsplit_once('.')?;
    let extension = extension.to_ascii_lowercase();
    BY_EXTENSION
        .iter()
        .find(|(known, _)| *known == extension)
        .map(|&(_, mime)| mime)
}

impl AllowedMime {
    pub(crate) fn resolve(declared: &str, path: &PagePath) -> Option<Self> {
        let essence = declared
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        if essence == UNTYPED {
            return mime_for_path(path).and_then(Self::parse);
        }
        Self::parse(&essence)
    }

    fn parse(essence: &str) -> Option<Self> {
        ALLOWED
            .iter()
            .find(|(allowed, _)| *allowed == essence)
            .map(|&(essence, kind)| Self { essence, kind })
    }

    pub(crate) fn as_str(&self) -> &'static str {
        self.essence
    }

    pub(crate) fn kind(&self) -> FileKind {
        self.kind
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resolve(declared: &str, path: &str) -> Option<AllowedMime> {
        AllowedMime::resolve(declared, &PagePath::parse(path).unwrap())
    }

    #[test]
    fn parameters_and_case_are_ignored() {
        let mime = resolve("Text/CSV; charset=utf-8", "a.csv").unwrap();
        assert_eq!(mime.as_str(), "text/csv");
        assert_eq!(mime.kind(), FileKind::Text);
    }

    #[test]
    fn untyped_bytes_take_the_type_of_their_extension() {
        let mime = resolve("application/octet-stream", "contracts/acme.DOCX").unwrap();
        assert_eq!(mime.kind(), FileKind::Document);
        assert_eq!(resolve("application/octet-stream", "a/noextension"), None);
        assert_eq!(resolve("application/octet-stream", "a.tar.gz"), None);
        assert_eq!(resolve("image/png", "a.md").unwrap().as_str(), "image/png");
    }

    #[test]
    fn markup_is_accepted() {
        for (declared, path) in [
            ("text/html", "mockups/home.html"),
            ("image/svg+xml", "brand/logo.svg"),
            ("text/xml", "tax/invoice.xml"),
            ("application/xml", "tax/invoice.xml"),
        ] {
            assert!(resolve(declared, path).is_some(), "{declared}");
        }
    }

    #[test]
    fn every_extension_maps_to_an_allowed_mime() {
        for (_, mime) in BY_EXTENSION {
            assert!(AllowedMime::parse(mime).is_some(), "{mime}");
        }
        let cases = [
            ("a/b.PDF", Some("application/pdf")),
            ("a/b.tar.gz", None),
            ("a/noextension", None),
            ("a.b/noextension", None),
        ];
        for (path, expected) in cases {
            let path = PagePath::parse(path).unwrap();
            assert_eq!(mime_for_path(&path), expected, "{path}");
        }
    }

    #[test]
    fn unknown_types_are_rejected() {
        for input in ["application/zip", "application/javascript", ""] {
            assert!(resolve(input, "a.zip").is_none(), "{input}");
        }
    }
}
