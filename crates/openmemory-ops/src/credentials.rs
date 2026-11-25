#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialPattern {
    PrivateKey,
    AwsAccessKey,
    GithubToken,
    AnthropicKey,
    OpenAiKey,
    SlackToken,
    GoogleApiKey,
    StripeKey,
    CardNumber,
    ResidentRegistrationNumber,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CredentialKind {
    Secret,
    PersonalData,
}

impl CredentialPattern {
    pub fn kind(self) -> CredentialKind {
        match self {
            Self::PrivateKey
            | Self::AwsAccessKey
            | Self::GithubToken
            | Self::AnthropicKey
            | Self::OpenAiKey
            | Self::SlackToken
            | Self::GoogleApiKey
            | Self::StripeKey => CredentialKind::Secret,
            Self::CardNumber | Self::ResidentRegistrationNumber => CredentialKind::PersonalData,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::PrivateKey => "private key",
            Self::AwsAccessKey => "aws access key",
            Self::GithubToken => "github token",
            Self::AnthropicKey => "anthropic api key",
            Self::OpenAiKey => "openai api key",
            Self::SlackToken => "slack token",
            Self::GoogleApiKey => "google api key",
            Self::StripeKey => "stripe key",
            Self::CardNumber => "card number",
            Self::ResidentRegistrationNumber => "resident registration number",
        }
    }
}

pub fn scan(text: &str) -> Option<CredentialPattern> {
    scan_secrets(text).or_else(|| scan_numbers(text))
}

pub fn scan_secrets(text: &str) -> Option<CredentialPattern> {
    scan_private_key(text).or_else(|| scan_tokens(text))
}

fn scan_private_key(text: &str) -> Option<CredentialPattern> {
    let begin = text.find("-----BEGIN ")?;
    text.get(begin..)?
        .lines()
        .next()?
        .contains("PRIVATE KEY-----")
        .then_some(CredentialPattern::PrivateKey)
}

fn scan_tokens(text: &str) -> Option<CredentialPattern> {
    text.split(|character: char| !(character.is_ascii_alphanumeric() || "_-".contains(character)))
        .find_map(classify_token)
}

fn classify_token(word: &str) -> Option<CredentialPattern> {
    let after = |prefix: &str, minimum: usize, accepts: fn(char) -> bool| {
        word.strip_prefix(prefix)
            .is_some_and(|rest| rest.chars().count() >= minimum && rest.chars().all(accepts))
    };
    if after("AKIA", 16, |c| c.is_ascii_uppercase() || c.is_ascii_digit())
        || after("ASIA", 16, |c| c.is_ascii_uppercase() || c.is_ascii_digit())
    {
        return Some(CredentialPattern::AwsAccessKey);
    }
    let github = ["ghp_", "gho_", "ghu_", "ghs_", "ghr_"]
        .iter()
        .any(|prefix| after(prefix, 30, |c| c.is_ascii_alphanumeric()))
        || after("github_pat_", 22, |c| c.is_ascii_alphanumeric() || c == '_');
    if github {
        return Some(CredentialPattern::GithubToken);
    }
    if after("sk-ant-", 20, |c| {
        c.is_ascii_alphanumeric() || c == '_' || c == '-'
    }) {
        return Some(CredentialPattern::AnthropicKey);
    }
    if after("sk-", 20, |c| {
        c.is_ascii_alphanumeric() || c == '_' || c == '-'
    }) {
        return Some(CredentialPattern::OpenAiKey);
    }
    let slack = ["xoxb-", "xoxp-", "xoxa-", "xoxr-", "xoxs-"]
        .iter()
        .any(|prefix| after(prefix, 10, |c| c.is_ascii_alphanumeric() || c == '-'));
    if slack {
        return Some(CredentialPattern::SlackToken);
    }
    if after("AIza", 35, |c| {
        c.is_ascii_alphanumeric() || c == '_' || c == '-'
    }) {
        return Some(CredentialPattern::GoogleApiKey);
    }
    let stripe = ["sk_live_", "rk_live_"]
        .iter()
        .any(|prefix| after(prefix, 16, |c| c.is_ascii_alphanumeric()));
    stripe.then_some(CredentialPattern::StripeKey)
}

struct DigitChain {
    groups: Vec<String>,
    separator_is_consistent: bool,
}

impl DigitChain {
    fn digits(&self) -> Vec<u32> {
        self.groups
            .iter()
            .flat_map(|group| group.chars().filter_map(|digit| digit.to_digit(10)))
            .collect()
    }

    fn has_resident_shape(&self) -> bool {
        let lengths: Vec<usize> = self.groups.iter().map(String::len).collect();
        lengths == [13] || (lengths == [6, 7] && self.separator_is_consistent)
    }

    fn has_card_shape(&self) -> bool {
        match self.groups.as_slice() {
            [single] => (13..=19).contains(&single.len()),
            groups => {
                self.separator_is_consistent
                    && groups.iter().all(|group| (4..=6).contains(&group.len()))
            }
        }
    }
}

fn scan_numbers(text: &str) -> Option<CredentialPattern> {
    for chain in digit_chains(text) {
        let digits = chain.digits();
        if chain.has_resident_shape() {
            if is_resident_registration_number(&digits) {
                return Some(CredentialPattern::ResidentRegistrationNumber);
            }
        } else if chain.has_card_shape()
            && (13..=19).contains(&digits.len())
            && matches!(digits.first(), Some(3..=6))
            && passes_luhn(&digits)
        {
            return Some(CredentialPattern::CardNumber);
        }
    }
    None
}

fn is_boundary_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '_'
}

fn digit_chains(text: &str) -> Vec<DigitChain> {
    let characters: Vec<char> = text.chars().collect();
    let mut chains = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        let starts_chain = characters[index].is_ascii_digit()
            && (index == 0 || !is_boundary_character(characters[index - 1]));
        if !starts_chain {
            index += 1;
            continue;
        }
        let mut groups = Vec::new();
        let mut separator = None;
        let mut separator_is_consistent = true;
        let mut end = index;
        loop {
            let start = end;
            while end < characters.len() && characters[end].is_ascii_digit() {
                end += 1;
            }
            groups.push(characters[start..end].iter().collect::<String>());
            let joins_next = end + 1 < characters.len()
                && matches!(characters[end], ' ' | '-')
                && characters[end + 1].is_ascii_digit();
            if !joins_next {
                break;
            }
            match separator {
                None => separator = Some(characters[end]),
                Some(existing) if existing != characters[end] => separator_is_consistent = false,
                Some(_) => {}
            }
            end += 1;
        }
        if end >= characters.len() || !is_boundary_character(characters[end]) {
            chains.push(DigitChain {
                groups,
                separator_is_consistent,
            });
        }
        index = end;
    }
    chains
}

fn passes_luhn(digits: &[u32]) -> bool {
    let sum: u32 = digits
        .iter()
        .rev()
        .enumerate()
        .map(|(position, digit)| {
            if position % 2 == 1 {
                let doubled = digit * 2;
                if doubled > 9 { doubled - 9 } else { doubled }
            } else {
                *digit
            }
        })
        .sum();
    sum.is_multiple_of(10)
}

fn is_resident_registration_number(digits: &[u32]) -> bool {
    let [_, _, m1, m2, d1, d2, gender, ..] = digits else {
        return false;
    };
    let month = m1 * 10 + m2;
    let day = d1 * 10 + d2;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || !(1..=8).contains(gender) {
        return false;
    }
    if is_corporate_registration_number(digits) {
        return false;
    }
    let weights = [2, 3, 4, 5, 6, 7, 8, 9, 2, 3, 4, 5];
    let sum: u32 = digits
        .iter()
        .zip(weights)
        .map(|(digit, weight)| digit * weight)
        .sum();
    let check = (11 - sum % 11) % 10;
    let last = digits.get(12).copied().unwrap_or(10);
    check == last || (*gender >= 5 && (check + 2) % 10 == last)
}

fn is_corporate_registration_number(digits: &[u32]) -> bool {
    let sum: u32 = digits
        .iter()
        .take(12)
        .enumerate()
        .map(|(position, digit)| digit * if position % 2 == 0 { 1 } else { 2 })
        .sum();
    digits.get(12).copied() == Some((10 - sum % 10) % 10)
}

#[cfg(test)]
mod tests;
