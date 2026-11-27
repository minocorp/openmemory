use std::fmt;
use std::str::FromStr;

use openmemory_ops::HitKind;
use time::OffsetDateTime;

use crate::files::FileId;
use crate::pages::ids::PageId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum Document {
    Page(PageId),
    File(FileId),
}

impl Document {
    pub(crate) fn kind(self) -> HitKind {
        match self {
            Self::Page(_) => HitKind::Page,
            Self::File(_) => HitKind::File,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct Position {
    at: i128,
    document: Document,
}

impl Position {
    pub(crate) fn new(at: OffsetDateTime, document: Document) -> Self {
        Self {
            at: at.unix_timestamp_nanos(),
            document,
        }
    }
}

impl fmt::Display for Position {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.document {
            Document::Page(id) => write!(formatter, "{}.p{}", self.at, id.0),
            Document::File(id) => write!(formatter, "{}.f{}", self.at, id.0),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("cursor is not valid")]
pub(crate) struct PositionParseError;

impl FromStr for Position {
    type Err = PositionParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let (at, document) = text.split_once('.').ok_or(PositionParseError)?;
        let at = at.parse().map_err(|_| PositionParseError)?;
        let document = if let Some(id) = document.strip_prefix('p') {
            Document::Page(PageId(id.parse().map_err(|_| PositionParseError)?))
        } else if let Some(id) = document.strip_prefix('f') {
            Document::File(FileId(id.parse().map_err(|_| PositionParseError)?))
        } else {
            return Err(PositionParseError);
        };
        Ok(Self { at, document })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_round_trip_through_text() {
        let at = OffsetDateTime::from_unix_timestamp(1_764_000_000).unwrap();
        for document in [Document::Page(PageId(42)), Document::File(FileId(7))] {
            let position = Position::new(at, document);
            assert_eq!(position.to_string().parse::<Position>(), Ok(position));
        }
    }

    #[test]
    fn malformed_cursors_are_rejected() {
        for text in ["", "12", "12.x3", "12.p", "a.p1", "12.p1.2"] {
            assert_eq!(text.parse::<Position>(), Err(PositionParseError), "{text}");
        }
    }

    #[test]
    fn later_documents_sort_after_earlier_ones() {
        let early = OffsetDateTime::from_unix_timestamp(1).unwrap();
        let late = OffsetDateTime::from_unix_timestamp(2).unwrap();
        assert!(
            Position::new(late, Document::Page(PageId(1)))
                > Position::new(early, Document::File(FileId(9)))
        );
        assert!(
            Position::new(early, Document::File(FileId(1)))
                > Position::new(early, Document::Page(PageId(9)))
        );
    }
}
