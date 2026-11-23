const PAGES_PREFIX: &str = "pages/";
const FILES_PREFIX: &str = "files/";
const PAGE_SUFFIX: &str = ".md";

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Entry<'a> {
    Page(&'a str),
    File(&'a str),
}

pub(super) fn page_entry(path: &str) -> String {
    format!("{PAGES_PREFIX}{path}{PAGE_SUFFIX}")
}

pub(super) fn file_entry(path: &str) -> String {
    format!("{FILES_PREFIX}{path}")
}

pub(super) fn classify(name: &str) -> Option<Entry<'_>> {
    if let Some(rest) = name.strip_prefix(PAGES_PREFIX) {
        return rest.strip_suffix(PAGE_SUFFIX).map(Entry::Page);
    }
    name.strip_prefix(FILES_PREFIX).map(Entry::File)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_names_round_trip_to_their_paths() {
        assert_eq!(
            classify(&page_entry("clients/acme")),
            Some(Entry::Page("clients/acme"))
        );
        assert_eq!(
            classify(&file_entry("contracts/acme.pdf")),
            Some(Entry::File("contracts/acme.pdf"))
        );
    }

    #[test]
    fn names_outside_the_layout_are_not_classified() {
        for name in ["readme.md", "pages/acme.txt", "other/acme.md"] {
            assert_eq!(classify(name), None, "{name}");
        }
    }
}
