use super::*;

// Fixtures are split with concat! so repository secret scanners do not block pushes.

#[test]
fn secrets_are_named_by_pattern() {
    let cases = [
        (
            "-----BEGIN OPENSSH PRIVATE KEY-----\nabc",
            CredentialPattern::PrivateKey,
        ),
        (
            "-----BEGIN RSA PRIVATE KEY-----",
            CredentialPattern::PrivateKey,
        ),
        (
            concat!("key AKIA", "ABCDEFGHIJKLMNOP end"),
            CredentialPattern::AwsAccessKey,
        ),
        (
            concat!("token ghp_", "abcdefghijklmnopqrstuvwxyz0123456789"),
            CredentialPattern::GithubToken,
        ),
        (
            "github_pat_11ABCDEFG0abcdefghijklmnop",
            CredentialPattern::GithubToken,
        ),
        (
            concat!("sk-ant-", "api03-abcdefghijklmnopqrstuvwx"),
            CredentialPattern::AnthropicKey,
        ),
        (
            "sk-proj-abcdefghijklmnopqrstuvwx",
            CredentialPattern::OpenAiKey,
        ),
        ("xoxb-123456789012-abcdef", CredentialPattern::SlackToken),
        (
            concat!("AIza", "SyA1234567890abcdefghijklmnopqrstuvw"),
            CredentialPattern::GoogleApiKey,
        ),
        (
            concat!("sk_live_", "abcdefghijklmnop1234"),
            CredentialPattern::StripeKey,
        ),
    ];
    for (text, expected) in cases {
        assert_eq!(scan(text), Some(expected), "{text}");
    }
}

#[test]
fn card_numbers_need_a_valid_luhn_checksum() {
    let cases = [
        ("card 4111111111111111", Some(CredentialPattern::CardNumber)),
        (
            "card 4111 1111 1111 1111",
            Some(CredentialPattern::CardNumber),
        ),
        (
            "card 4111-1111-1111-1111",
            Some(CredentialPattern::CardNumber),
        ),
        ("card 5500005555555559", Some(CredentialPattern::CardNumber)),
        ("card 4111111111111112", None),
        ("order 2026-11-23 2026-11-24", None),
        ("phone 010-1234-5678", None),
        ("id x4111111111111111", None),
    ];
    for (text, expected) in cases {
        assert_eq!(scan(text), expected, "{text}");
    }
}

#[test]
fn resident_numbers_need_a_valid_checksum_and_exempt_corporate_numbers() {
    let cases = [
        (
            "rrn 900101-1234568",
            Some(CredentialPattern::ResidentRegistrationNumber),
        ),
        (
            "rrn 9001011234568",
            Some(CredentialPattern::ResidentRegistrationNumber),
        ),
        ("rrn 900101-1234567", None),
        ("rrn 901301-1234568", None),
        ("법인 110111-0000002", None),
        ("법인 900101-1000006", None),
        (
            "번호900101-1234568",
            Some(CredentialPattern::ResidentRegistrationNumber),
        ),
    ];
    for (text, expected) in cases {
        assert_eq!(scan(text), expected, "{text}");
    }
}

#[test]
fn ordinary_text_passes() {
    let cases = [
        "Design outsourcing partner. 450,000 KRW per month.",
        "call 2026-11-20 about sk-short",
        "## Timeline\n2026-03-02 | call — signed, 500,000",
        "BEGIN PRIVATE KEY is a phrase",
        "",
    ];
    for text in cases {
        assert_eq!(scan(text), None, "{text}");
    }
}

#[test]
fn patterns_are_split_into_secrets_and_personal_data() {
    assert_eq!(CredentialPattern::PrivateKey.kind(), CredentialKind::Secret);
    assert_eq!(CredentialPattern::StripeKey.kind(), CredentialKind::Secret);
    assert_eq!(
        CredentialPattern::CardNumber.kind(),
        CredentialKind::PersonalData
    );
    assert_eq!(
        CredentialPattern::ResidentRegistrationNumber.kind(),
        CredentialKind::PersonalData
    );
}

#[test]
fn scanning_for_secrets_ignores_personal_data() {
    assert_eq!(scan_secrets("card 4111 1111 1111 1111"), None);
    assert_eq!(scan_secrets("rrn 900101-1234568"), None);
    assert_eq!(
        scan_secrets(concat!("key AKIA", "ABCDEFGHIJKLMNOP end")),
        Some(CredentialPattern::AwsAccessKey)
    );
    assert_eq!(
        scan_secrets("-----BEGIN PRIVATE KEY-----"),
        Some(CredentialPattern::PrivateKey)
    );
}
