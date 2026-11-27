use time::OffsetDateTime;

use crate::search::position::{Document, Position};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Candidate {
    pub(crate) document: Document,
    pub(crate) path: String,
    pub(crate) author: String,
    pub(crate) at: OffsetDateTime,
}

impl Candidate {
    pub(crate) fn position(&self) -> Position {
        Position::new(self.at, self.document)
    }
}
