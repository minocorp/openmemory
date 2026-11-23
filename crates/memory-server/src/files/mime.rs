use memory_ops::PagePath;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum FileKind {
    Pdf,
    Text,
    Image,
    Spreadsheet,
    Audio,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct AllowedMime {
    essence: String,
    kind: FileKind,
}

const ALLOWED: &[(&str, FileKind)] = &[
    ("application/pdf", FileKind::Pdf),
    ("text/plain", FileKind::Text),
    ("text/markdown", FileKind::Text),
    ("text/csv", FileKind::Text),
    ("image/png", FileKind::Image),
    ("image/jpeg", FileKind::Image),
    ("image/gif", FileKind::Image),
    ("image/webp", FileKind::Image),
    ("image/heic", FileKind::Image),
    ("image/avif", FileKind::Image),
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
];

const BY_EXTENSION: &[(&str, &str)] = &[
    ("pdf", "application/pdf"),
    ("txt", "text/plain"),
    ("md", "text/markdown"),
    ("csv", "text/csv"),
    ("png", "image/png"),
    ("jpg", "image/jpeg"),
    ("jpeg", "image/jpeg"),
    ("gif", "image/gif"),
    ("webp", "image/webp"),
    ("heic", "image/heic"),
    ("avif", "image/avif"),
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
    pub(crate) fn parse(input: &str) -> Option<Self> {
        let essence = input
            .split(';')
            .next()
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        ALLOWED
            .iter()
            .find(|(allowed, _)| *allowed == essence)
            .map(|&(_, kind)| Self { essence, kind })
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.essence
    }

    pub(crate) fn kind(&self) -> FileKind {
        self.kind
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameters_and_case_are_ignored() {
        let mime = AllowedMime::parse("Text/CSV; charset=utf-8").unwrap();
        assert_eq!(mime.as_str(), "text/csv");
        assert_eq!(mime.kind(), FileKind::Text);
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
    fn scriptable_and_unknown_types_are_rejected() {
        for input in ["image/svg+xml", "text/html", "application/zip", ""] {
            assert!(AllowedMime::parse(input).is_none(), "{input}");
        }
    }
}
