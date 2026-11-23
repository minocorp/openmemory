use memory_ops::PagePath;

use crate::access::{Folders, Scope};

const ALL_FOLDERS: &str = "*";

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum ScopesError {
    #[error("scopes are empty")]
    Empty,
    #[error("scopes mix the all-folders marker with folders")]
    MixedMarker,
    #[error("scope is not a valid folder path")]
    InvalidFolder,
}

pub(crate) fn encode(folders: &Folders) -> Vec<String> {
    match folders {
        Folders::All => vec![ALL_FOLDERS.to_owned()],
        Folders::Only(prefixes) => prefixes.iter().map(ToString::to_string).collect(),
    }
}

pub(crate) fn decode(scopes: &[String]) -> Result<Folders, ScopesError> {
    if scopes.is_empty() {
        return Err(ScopesError::Empty);
    }
    if scopes.iter().any(|scope| scope == ALL_FOLDERS) {
        return if scopes.len() == 1 {
            Ok(Folders::All)
        } else {
            Err(ScopesError::MixedMarker)
        };
    }
    scopes
        .iter()
        .map(|scope| PagePath::parse(scope).map_err(|_| ScopesError::InvalidFolder))
        .collect::<Result<Vec<_>, _>>()
        .map(Folders::Only)
}

pub(crate) fn to_scope(scopes: &[String], can_write: bool) -> Result<Scope, ScopesError> {
    Ok(Scope {
        folders: decode(scopes)?,
        can_write,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn text(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    #[test]
    fn all_folders_round_trips_as_a_single_star() {
        assert_eq!(encode(&Folders::All), text(&["*"]));
        assert_eq!(decode(&text(&["*"])), Ok(Folders::All));
    }

    #[test]
    fn folder_prefixes_round_trip() {
        let folders = Folders::Only(vec![PagePath::parse("tax").unwrap()]);
        assert_eq!(decode(&encode(&folders)), Ok(folders));
    }

    #[test]
    fn invalid_encodings_are_rejected_by_cause() {
        assert_eq!(decode(&[]), Err(ScopesError::Empty));
        assert_eq!(decode(&text(&["*", "tax"])), Err(ScopesError::MixedMarker));
        assert_eq!(decode(&text(&["a//b"])), Err(ScopesError::InvalidFolder));
    }
}
