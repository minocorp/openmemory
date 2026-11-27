use openmemory_ops::escape_like;
use unicode_normalization::UnicodeNormalization;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct Terms(Vec<String>);

impl Terms {
    pub(crate) fn parse(text: Option<&str>) -> Self {
        let mut words: Vec<String> = Vec::new();
        for word in text.unwrap_or_default().split_whitespace().map(fold) {
            if !words.contains(&word) {
                words.push(word);
            }
        }
        Self(words)
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn words(&self) -> &[String] {
        &self.0
    }

    pub(crate) fn like_patterns(&self) -> Vec<String> {
        self.0
            .iter()
            .map(|word| format!("%{}%", escape_like(word)))
            .collect()
    }

    pub(crate) fn matching_lines(&self, text: &str, max: usize) -> Vec<(u32, String)> {
        if self.0.is_empty() {
            return Vec::new();
        }
        text.lines()
            .zip(1_u32..)
            .filter(|(line, _)| {
                let folded = fold(line);
                self.0.iter().any(|word| folded.contains(word.as_str()))
            })
            .map(|(line, number)| (number, line.to_owned()))
            .take(max)
            .collect()
    }
}

// Mirrors the SQL function search_text so the lines shown are the lines that matched.
fn fold(text: &str) -> String {
    text.nfc().collect::<String>().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn words_are_split_folded_and_listed_once() {
        let terms = Terms::parse(Some("  Fee  계약\tfee "));
        assert_eq!(terms.words(), ["fee", "계약"]);
        assert!(Terms::parse(Some("   ")).is_empty());
        assert!(Terms::parse(None).is_empty());
    }

    #[test]
    fn decomposed_hangul_folds_to_the_composed_form() {
        let decomposed: String = "계약".nfd().collect();
        assert_eq!(Terms::parse(Some(&decomposed)).words(), ["계약"]);
    }

    #[test]
    fn like_patterns_escape_wildcards() {
        let terms = Terms::parse(Some("100% a_b"));
        assert_eq!(terms.like_patterns(), ["%100\\%%", "%a\\_b%"]);
    }

    #[test]
    fn lines_matching_any_word_are_numbered_from_one() {
        let terms = Terms::parse(Some("beta 한글"));
        let text = "alpha BETA\nnothing\n\n한글 검색\nbeta";
        assert_eq!(
            terms.matching_lines(text, 5),
            [
                (1, "alpha BETA".to_owned()),
                (4, "한글 검색".to_owned()),
                (5, "beta".to_owned())
            ]
        );
        assert_eq!(terms.matching_lines(text, 1).len(), 1);
        assert_eq!(Terms::default().matching_lines(text, 5).len(), 0);
    }
}
