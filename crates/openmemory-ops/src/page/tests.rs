use time::{Date, Month};

use super::*;

const SAMPLE: &str = "---\nstatus: active\nrenewal: 2027-03-01\nfee: 450000\ntags: [design, 외주]\n---\nDesign outsourcing partner. 450,000 KRW per month.\n\n## Timeline\n2026-11-20 | call — agreed to lower the fee to 450,000\n2026-03-02 | contracts/acme.pdf#p2 \"월 금 오십만원정\" — contract signed, 500,000\n";

fn date(day: u8) -> Date {
    Date::from_calendar_date(2026, Month::November, day).unwrap()
}

#[test]
fn unchanged_text_round_trips_byte_for_byte() {
    let cases = [
        SAMPLE,
        "",
        "plain body without anything",
        "plain body\n",
        "---\n---\nbody\n",
        "---\r\nstatus: a\r\n---\r\nbody\r\n",
        "body\n\n## Timeline\n",
        "body\n\n## Timeline",
        "body\n\n## Timeline\n2026-11-20 | call — text\n\n\n",
        "## Timeline\n2026-11-20 | call — text",
    ]
    .map(str::to_owned);
    for text in cases {
        assert_eq!(Page::parse(&text).unwrap().to_text(), text, "{text:?}");
    }
}

#[test]
fn sections_are_separated() {
    let page = Page::parse(SAMPLE).unwrap();
    assert_eq!(
        page.body(),
        "Design outsourcing partner. 450,000 KRW per month.\n\n"
    );
    assert_eq!(page.timeline().len(), 2);
    assert_eq!(
        page.timeline()[0].text(),
        "agreed to lower the fee to 450,000"
    );
}

#[test]
fn fields_are_typed_by_their_text() {
    let page = Page::parse(SAMPLE).unwrap();
    let fields = page.fields();
    assert_eq!(fields[0].values(), [FieldValue::Text("active".to_owned())]);
    assert_eq!(
        fields[1].values()[0].date(),
        Some(date_of(2027, Month::March, 1))
    );
    assert_eq!(fields[2].values()[0].number(), Some("450000"));
    assert_eq!(
        fields[3]
            .values()
            .iter()
            .map(FieldValue::text)
            .collect::<Vec<_>>(),
        ["design", "외주"]
    );
}

fn date_of(year: i32, month: Month, day: u8) -> Date {
    Date::from_calendar_date(year, month, day).unwrap()
}

#[test]
fn quoted_and_numeric_strings_keep_their_text() {
    let page = Page::parse("---\nzip: \"01234\"\nflag: true\nratio: 1.5\nnote:\n---\n").unwrap();
    let texts: Vec<Vec<&str>> = page
        .fields()
        .iter()
        .map(|field| field.values().iter().map(FieldValue::text).collect())
        .collect();
    assert_eq!(texts, [vec!["01234"], vec!["true"], vec!["1.5"], vec![]]);
}

#[test]
fn grammar_breaks_are_rejected_with_line_numbers() {
    let cases = [
        (
            "---\nstatus: a\nbody",
            1,
            PageErrorKind::UnclosedFrontmatter,
        ),
        (
            "---\nnested:\n  a: 1\n---\n",
            2,
            PageErrorKind::FieldNotScalar {
                key: "nested".to_owned(),
            },
        ),
        (
            "---\na: 1\nb: [[1], 2]\n---\n",
            3,
            PageErrorKind::FieldNotScalar {
                key: "b".to_owned(),
            },
        ),
        (
            "---\n: x\n---\n",
            2,
            PageErrorKind::Frontmatter {
                reason: String::new(),
            },
        ),
        (
            "body\n## Timeline\nnot a line\n",
            3,
            PageErrorKind::TimelineDate,
        ),
        (
            "## Timeline\n2026-11-20 | call — a\n\n2026-11-21 | call — b\n",
            3,
            PageErrorKind::TimelineBlankLine,
        ),
        (
            "x\n\n## Timeline\n2026-11-20 call — a\n",
            4,
            PageErrorKind::TimelineSeparator,
        ),
    ];
    for (text, line, kind) in cases {
        let error = Page::parse(text).unwrap_err();
        assert_eq!(error.line(), line, "{text:?}");
        assert_eq!(
            std::mem::discriminant(error.kind()),
            std::mem::discriminant(&kind),
            "{text:?}"
        );
    }
}

#[test]
fn oversized_field_values_and_keys_are_rejected() {
    let long_value = format!("---\nnote: {}\n---\n", "a".repeat(2001));
    let long_key = format!("---\n{}: x\n---\n", "k".repeat(65));
    assert!(matches!(
        Page::parse(&long_value).unwrap_err().kind(),
        PageErrorKind::FieldValueTooLong { .. }
    ));
    assert!(matches!(
        Page::parse(&long_key).unwrap_err().kind(),
        PageErrorKind::FieldKeyLength { .. }
    ));
}

#[test]
fn only_the_final_timeline_heading_starts_the_timeline() {
    let page = Page::parse("a\n## Timeline\n2026-11-20 | x — y\n## Timeline\n2026-11-21 | x — z\n")
        .unwrap();
    assert_eq!(page.body(), "a\n## Timeline\n2026-11-20 | x — y\n");
    assert_eq!(page.timeline().len(), 1);
}

#[test]
fn appending_adds_a_line_without_touching_the_rest() {
    let mut page = Page::parse(SAMPLE).unwrap();
    page.append_timeline(date(23), "mino/claude-code", "renewed")
        .unwrap();
    assert_eq!(
        page.to_text(),
        format!("{SAMPLE}2026-11-23 | mino/claude-code — renewed\n")
    );
    assert_eq!(Page::parse(&page.to_text()).unwrap(), page);
}

#[test]
fn appending_creates_the_timeline_section_when_missing() {
    let cases = [
        ("body", "body\n\n## Timeline\n2026-11-23 | call — hi\n"),
        ("body\n", "body\n\n## Timeline\n2026-11-23 | call — hi\n"),
        ("body\n\n", "body\n\n## Timeline\n2026-11-23 | call — hi\n"),
        ("", "## Timeline\n2026-11-23 | call — hi\n"),
        (
            "body\n\n## Timeline",
            "body\n\n## Timeline\n2026-11-23 | call — hi\n",
        ),
        (
            "body\n\n## Timeline\n\n\n",
            "body\n\n## Timeline\n2026-11-23 | call — hi\n",
        ),
    ];
    for (text, expected) in cases {
        let mut page = Page::parse(text).unwrap();
        page.append_timeline(date(23), "call", "hi").unwrap();
        assert_eq!(page.to_text(), expected, "{text:?}");
    }
}

#[test]
fn appended_lines_are_validated() {
    let cases = [
        ("", "text", Err(PageErrorKind::TimelineSource)),
        ("call", "", Err(PageErrorKind::TimelineText)),
        ("a — b", "text", Err(PageErrorKind::TimelineSource)),
        ("call", "two\nlines", Err(PageErrorKind::TimelineText)),
        ("ca\nll", "text", Err(PageErrorKind::TimelineSource)),
        ("contracts/a.pdf#p2 \"q — r\"", "text", Ok(())),
        ("call", "text — with dash", Ok(())),
    ];
    for (source, text, expected) in cases {
        let mut page = Page::parse("body\n").unwrap();
        let result = page
            .append_timeline(date(23), source, text)
            .map_err(|error| error.kind().clone());
        assert_eq!(result, expected, "{source:?} {text:?}");
    }
}

#[test]
fn timeline_may_only_grow_by_appending() {
    let before = Page::parse(SAMPLE).unwrap();
    let appended = Page::parse(&format!("{SAMPLE}2026-11-23 | call — more\n")).unwrap();
    let rewritten = Page::parse(&SAMPLE.replace("lower", "raise")).unwrap();
    let removed = Page::parse(&SAMPLE.replace(
        "2026-03-02 | contracts/acme.pdf#p2 \"월 금 오십만원정\" — contract signed, 500,000\n",
        "",
    ))
    .unwrap();
    let reordered_body =
        Page::parse(&SAMPLE.replace("Design outsourcing", "Design studio")).unwrap();
    let no_timeline = Page::parse("body").unwrap();
    assert!(appended.extends_timeline_of(&before));
    assert!(reordered_body.extends_timeline_of(&before));
    assert!(!rewritten.extends_timeline_of(&before));
    assert!(!removed.extends_timeline_of(&before));
    assert!(!no_timeline.extends_timeline_of(&before));
    assert!(before.extends_timeline_of(&no_timeline));
}
