use std::fmt;
use std::str::FromStr;

use crate::pages::ids::Seq;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Epoch {
    pub(super) system: i64,
    pub(super) timeline: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Cursor {
    pub(super) epoch: Epoch,
    pub(super) seq: Seq,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Position {
    pub(super) epoch: Epoch,
    pub(super) head: Seq,
}

impl Cursor {
    pub(super) fn at(self, seq: Seq) -> Self {
        Self { seq, ..self }
    }

    pub(super) fn continues(self, position: Position) -> bool {
        self.epoch == position.epoch && self.seq <= position.head
    }
}

impl Position {
    pub(super) fn cursor(self) -> Cursor {
        Cursor {
            epoch: self.epoch,
            seq: self.head,
        }
    }
}

impl fmt::Display for Cursor {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}.{}.{}",
            self.epoch.system, self.epoch.timeline, self.seq.0
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid stream cursor")]
pub(crate) struct CursorParseError;

impl FromStr for Cursor {
    type Err = CursorParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut parts = text.split('.');
        let (Some(system), Some(timeline), Some(seq), None) =
            (parts.next(), parts.next(), parts.next(), parts.next())
        else {
            return Err(CursorParseError);
        };
        Ok(Self {
            epoch: Epoch {
                system: system.parse().map_err(|_| CursorParseError)?,
                timeline: timeline.parse().map_err(|_| CursorParseError)?,
            },
            seq: Seq(seq.parse().map_err(|_| CursorParseError)?),
        })
    }
}
