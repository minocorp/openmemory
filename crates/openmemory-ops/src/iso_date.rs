use time::{Date, Month};

pub fn parse_iso_date(text: &str) -> Option<Date> {
    let bytes = text.as_bytes();
    if bytes.len() != 10 || bytes.get(4) != Some(&b'-') || bytes.get(7) != Some(&b'-') {
        return None;
    }
    let year = digits(text.get(0..4)?)?;
    let month = digits(text.get(5..7)?)?;
    let day = digits(text.get(8..10)?)?;
    let month = Month::try_from(u8::try_from(month).ok()?).ok()?;
    Date::from_calendar_date(i32::try_from(year).ok()?, month, u8::try_from(day).ok()?).ok()
}

pub fn format_iso_date(date: Date) -> String {
    format!(
        "{:04}-{:02}-{:02}",
        date.year(),
        u8::from(date.month()),
        date.day()
    )
}

fn digits(text: &str) -> Option<u32> {
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    text.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dates_round_trip_and_invalid_dates_are_rejected() {
        let cases = [
            ("2026-11-20", true),
            ("2026-02-30", false),
            ("2026-13-01", false),
            ("2026-1-01", false),
            ("2026/11/20", false),
            ("+026-11-20", false),
            ("", false),
        ];
        for (input, valid) in cases {
            let parsed = parse_iso_date(input);
            assert_eq!(parsed.is_some(), valid, "{input}");
            if let Some(date) = parsed {
                assert_eq!(format_iso_date(date), input);
            }
        }
    }
}
