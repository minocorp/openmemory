use memory_ops::escape_like;

use crate::access::{Folders, Scope};

#[derive(Debug, Clone)]
pub(crate) struct ScopeFilter {
    pub(crate) unrestricted: bool,
    pub(crate) exact: Vec<String>,
    pub(crate) patterns: Vec<String>,
}

impl ScopeFilter {
    pub(crate) fn of(scope: &Scope) -> Self {
        match &scope.folders {
            Folders::All => Self {
                unrestricted: true,
                exact: Vec::new(),
                patterns: Vec::new(),
            },
            Folders::Only(prefixes) => Self {
                unrestricted: false,
                exact: prefixes
                    .iter()
                    .map(|prefix| prefix.as_str().to_owned())
                    .collect(),
                patterns: prefixes
                    .iter()
                    .map(|prefix| format!("{}/%", escape_like(prefix.as_str())))
                    .collect(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use memory_ops::PagePath;

    use super::*;

    #[test]
    fn prefix_scopes_match_the_folder_itself_and_its_children() {
        let scope = Scope {
            folders: Folders::Only(vec![PagePath::parse("tax_2026").unwrap()]),
            can_write: true,
        };
        let filter = ScopeFilter::of(&scope);
        assert!(!filter.unrestricted);
        assert_eq!(filter.exact, ["tax_2026"]);
        assert_eq!(filter.patterns, ["tax\\_2026/%"]);
    }

    #[test]
    fn everything_scope_is_unrestricted() {
        assert!(ScopeFilter::of(&Scope::everything()).unrestricted);
    }
}
