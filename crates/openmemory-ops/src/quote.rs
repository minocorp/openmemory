use unicode_normalization::UnicodeNormalization;

pub fn quote_occurs(text: &str, quote: &str) -> bool {
    let quote = normalize(quote);
    !quote.is_empty() && normalize(text).contains(&quote)
}

fn normalize(text: &str) -> String {
    text.nfc()
        .collect::<String>()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitespace_unicode_form_and_latin_case_do_not_matter() {
        let cases = [
            (
                "Monthly fee:\n  500,000 KRW",
                "monthly fee: 500,000 krw",
                true,
            ),
            ("월  금\n오십만원정", "월 금 오십만원정", true),
            ("\u{1112}\u{1161}\u{11ab}", "한", true),
            ("fee 500,000", "fee 450,000", false),
            ("fee 500,000", "   ", false),
        ];
        for (text, quote, expected) in cases {
            assert_eq!(quote_occurs(text, quote), expected, "{quote}");
        }
    }
}
