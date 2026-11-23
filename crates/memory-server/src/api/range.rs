use std::ops::Range;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RangeRequest {
    Whole,
    Partial(Range<u64>),
    Unsatisfiable,
}

pub(super) fn parse(header: Option<&str>, size: u64) -> RangeRequest {
    let Some(spec) = header.and_then(|value| value.trim().strip_prefix("bytes=")) else {
        return RangeRequest::Whole;
    };
    if spec.contains(',') {
        return RangeRequest::Whole;
    }
    let Some((first, last)) = spec.trim().split_once('-') else {
        return RangeRequest::Whole;
    };
    match (first.trim(), last.trim()) {
        ("", suffix) => suffix_range(suffix, size),
        (start, "") => open_range(start, size),
        (start, end) => closed_range(start, end, size),
    }
}

fn suffix_range(suffix: &str, size: u64) -> RangeRequest {
    let Ok(length) = suffix.parse::<u64>() else {
        return RangeRequest::Whole;
    };
    if length == 0 || size == 0 {
        return RangeRequest::Unsatisfiable;
    }
    RangeRequest::Partial(size.saturating_sub(length)..size)
}

fn open_range(start: &str, size: u64) -> RangeRequest {
    let Ok(start) = start.parse::<u64>() else {
        return RangeRequest::Whole;
    };
    if start >= size {
        return RangeRequest::Unsatisfiable;
    }
    RangeRequest::Partial(start..size)
}

fn closed_range(start: &str, end: &str, size: u64) -> RangeRequest {
    let (Ok(start), Ok(end)) = (start.parse::<u64>(), end.parse::<u64>()) else {
        return RangeRequest::Whole;
    };
    if start > end {
        return RangeRequest::Whole;
    }
    if start >= size {
        return RangeRequest::Unsatisfiable;
    }
    RangeRequest::Partial(start..end.min(size - 1) + 1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_are_resolved_against_the_size() {
        let cases = [
            (Some("bytes=0-9"), RangeRequest::Partial(0..10)),
            (Some("bytes=10-"), RangeRequest::Partial(10..100)),
            (Some("bytes=-20"), RangeRequest::Partial(80..100)),
            (Some("bytes=90-500"), RangeRequest::Partial(90..100)),
            (Some("bytes=-500"), RangeRequest::Partial(0..100)),
            (Some("bytes=100-"), RangeRequest::Unsatisfiable),
            (Some("bytes=200-300"), RangeRequest::Unsatisfiable),
            (Some("bytes=-0"), RangeRequest::Unsatisfiable),
        ];
        for (header, expected) in cases {
            assert_eq!(parse(header, 100), expected, "{header:?}");
        }
    }

    #[test]
    fn unsupported_or_malformed_headers_serve_the_whole_file() {
        let cases = [
            None,
            Some("items=0-5"),
            Some("bytes=0-5,10-15"),
            Some("bytes=9-3"),
            Some("bytes=a-b"),
            Some("bytes=5"),
        ];
        for header in cases {
            assert_eq!(parse(header, 100), RangeRequest::Whole, "{header:?}");
        }
    }
}
