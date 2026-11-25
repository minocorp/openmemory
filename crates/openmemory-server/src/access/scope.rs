use openmemory_ops::PagePath;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Folders {
    All,
    Only(Vec<PagePath>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Scope {
    pub(crate) folders: Folders,
    pub(crate) can_write: bool,
}

impl Scope {
    pub(crate) fn everything() -> Self {
        Self {
            folders: Folders::All,
            can_write: true,
        }
    }

    pub(crate) fn can_read(&self, path: &PagePath) -> bool {
        match &self.folders {
            Folders::All => true,
            Folders::Only(prefixes) => prefixes.iter().any(|prefix| contains(prefix, path)),
        }
    }

    pub(crate) fn can_write(&self, path: &PagePath) -> bool {
        self.can_write && self.can_read(path)
    }
}

fn contains(prefix: &PagePath, path: &PagePath) -> bool {
    let prefix = prefix.as_str();
    let path = path.as_str();
    path == prefix
        || path
            .strip_prefix(prefix)
            .is_some_and(|rest| rest.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(value: &str) -> PagePath {
        PagePath::parse(value).unwrap()
    }

    #[test]
    fn prefixes_match_on_segment_boundaries() {
        let scope = Scope {
            folders: Folders::Only(vec![path("tax")]),
            can_write: true,
        };
        assert!(scope.can_read(&path("tax")));
        assert!(scope.can_read(&path("tax/2026")));
        assert!(!scope.can_read(&path("taxes/2026")));
    }

    #[test]
    fn read_only_scope_cannot_write() {
        let scope = Scope {
            can_write: false,
            ..Scope::everything()
        };
        assert!(scope.can_read(&path("customers/acme")));
        assert!(!scope.can_write(&path("customers/acme")));
    }
}
