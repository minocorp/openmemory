pub fn escape_like(text: &str) -> String {
    let mut escaped = String::with_capacity(text.len());
    for character in text.chars() {
        if matches!(character, '\\' | '%' | '_') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcards_and_escape_characters_are_escaped() {
        let cases = [
            ("plain", "plain"),
            ("50%", "50\\%"),
            ("a_b", "a\\_b"),
            ("a\\b", "a\\\\b"),
            ("한글%_", "한글\\%\\_"),
        ];
        for (input, expected) in cases {
            assert_eq!(escape_like(input), expected);
        }
    }
}
