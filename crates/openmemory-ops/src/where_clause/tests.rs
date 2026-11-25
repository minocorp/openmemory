use time::Month;

use super::*;

fn parse_one(input: &str) -> Result<Condition, WhereErrorKind> {
    WhereClause::parse(&[input.to_owned()])
        .map(|clause| clause.conditions[0].clone())
        .map_err(|error| error.kind)
}

fn compare(key: &str, comparison: Comparison, operand: Operand) -> Condition {
    Condition {
        key: key.to_owned(),
        test: Test::Compare(comparison, operand),
    }
}

#[test]
fn conditions_parse_by_operator_and_value_type() {
    let date = Date::from_calendar_date(2027, Month::March, 1).unwrap();
    let cases = [
        (
            "status=active",
            compare("status", Comparison::Equal, Operand::Text("active".into())),
        ),
        (
            "status!=active",
            compare(
                "status",
                Comparison::NotEqual,
                Operand::Text("active".into()),
            ),
        ),
        (
            "fee<450000",
            compare("fee", Comparison::Less, Operand::Number("450000".into())),
        ),
        (
            "fee>-1.5",
            compare("fee", Comparison::Greater, Operand::Number("-1.5".into())),
        ),
        (
            "renewal<2027-03-01",
            compare("renewal", Comparison::Less, Operand::Date(date)),
        ),
        (
            "renewal<today",
            compare("renewal", Comparison::Less, Operand::DaysFromToday(0)),
        ),
        (
            "renewal<+90d",
            compare("renewal", Comparison::Less, Operand::DaysFromToday(90)),
        ),
        (
            "renewal>-30d",
            compare("renewal", Comparison::Greater, Operand::DaysFromToday(-30)),
        ),
        (
            "name~Kim Min",
            compare(
                "name",
                Comparison::Contains,
                Operand::Text("Kim Min".into()),
            ),
        ),
        (
            "name~today",
            compare("name", Comparison::Contains, Operand::Text("today".into())),
        ),
        (
            "tags=a=b",
            compare("tags", Comparison::Equal, Operand::Text("a=b".into())),
        ),
        (
            " status = active ",
            compare("status", Comparison::Equal, Operand::Text("active".into())),
        ),
        (
            "renewal:exists",
            Condition {
                key: "renewal".into(),
                test: Test::Exists,
            },
        ),
    ];
    for (input, expected) in cases {
        assert_eq!(parse_one(input), Ok(expected), "{input}");
    }
}

#[test]
fn invalid_conditions_are_rejected_by_cause() {
    let cases = [
        ("status", WhereErrorKind::UnknownOperator),
        ("=active", WhereErrorKind::MissingKey),
        ("status=", WhereErrorKind::MissingValue),
        ("status~ ", WhereErrorKind::MissingValue),
        ("status:missing", WhereErrorKind::UnknownOperator),
        ("status!active", WhereErrorKind::UnknownOperator),
        ("status:exists now", WhereErrorKind::UnknownOperator),
    ];
    for (input, expected) in cases {
        assert_eq!(parse_one(input), Err(expected), "{input}");
    }
}

#[test]
fn resolved_conditions_read_back_in_plain_words() {
    let today = Date::from_calendar_date(2026, Month::November, 23).unwrap();
    let clause = WhereClause::parse(&[
        "renewal<+90d".to_owned(),
        "fee>450000".to_owned(),
        "status!=active".to_owned(),
        "name~Kim".to_owned(),
        "owner:exists".to_owned(),
    ])
    .unwrap();
    let described: Vec<String> = clause
        .resolve(today)
        .unwrap()
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        described,
        [
            "renewal < date 2027-02-21",
            "fee > number 450000",
            "status != text \"active\"",
            "name contains text \"Kim\"",
            "owner exists",
        ]
    );
}

#[test]
fn errors_name_the_failing_condition() {
    let error = WhereClause::parse(&["a=1".to_owned(), "b".to_owned()]).unwrap_err();
    assert_eq!(error.index(), 1);
}

#[test]
fn relative_dates_resolve_against_today() {
    let today = Date::from_calendar_date(2026, Month::November, 23).unwrap();
    let cases = [
        (Operand::DaysFromToday(0), (2026, Month::November, 23)),
        (Operand::DaysFromToday(90), (2027, Month::February, 21)),
        (Operand::DaysFromToday(-30), (2026, Month::October, 24)),
    ];
    for (operand, (year, month, day)) in cases {
        let expected = Date::from_calendar_date(year, month, day).unwrap();
        assert_eq!(operand.resolve(today), Ok(Value::Date(expected)));
    }
    assert_eq!(
        Operand::DaysFromToday(i64::MAX).resolve(today),
        Err(WhereErrorKind::DateOutOfRange)
    );
}
