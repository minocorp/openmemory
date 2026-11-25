use serde::Deserialize;
use serde::de::{self, MapAccess, SeqAccess, Visitor};
use time::Date;

use crate::iso_date::parse_iso_date;
use crate::page::error::PageErrorKind;

const MAX_KEY_LENGTH: usize = 64;
const MAX_VALUE_BYTES: usize = 2000;

pub(crate) type FieldFailure = (Option<usize>, PageErrorKind);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Field {
    key: String,
    values: Vec<FieldValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FieldValue {
    Text(String),
    Number(String),
    Date(Date, String),
}

impl Field {
    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn values(&self) -> &[FieldValue] {
        &self.values
    }
}

impl FieldValue {
    pub fn text(&self) -> &str {
        match self {
            Self::Text(text) | Self::Number(text) | Self::Date(_, text) => text,
        }
    }

    pub fn number(&self) -> Option<&str> {
        match self {
            Self::Number(text) => Some(text),
            Self::Text(_) | Self::Date(..) => None,
        }
    }

    pub fn date(&self) -> Option<Date> {
        match self {
            Self::Date(date, _) => Some(*date),
            Self::Text(_) | Self::Number(_) => None,
        }
    }

    pub(crate) fn classify(text: &str) -> Self {
        if let Some(date) = parse_iso_date(text) {
            Self::Date(date, text.to_owned())
        } else if is_number(text) {
            Self::Number(text.to_owned())
        } else {
            Self::Text(text.to_owned())
        }
    }
}

pub(crate) fn is_number(text: &str) -> bool {
    let unsigned = text.strip_prefix('-').unwrap_or(text);
    let (whole, fraction) = unsigned.split_once('.').unwrap_or((unsigned, "0"));
    let digits = |part: &str| !part.is_empty() && part.bytes().all(|byte| byte.is_ascii_digit());
    digits(whole) && digits(fraction)
}

pub(crate) fn parse_fields(yaml: &str) -> Result<Vec<Field>, FieldFailure> {
    let entries = serde_saphyr::from_str::<Entries>(yaml).map_err(|error| {
        let line = error
            .location()
            .and_then(|location| usize::try_from(location.line()).ok());
        (
            line,
            PageErrorKind::Frontmatter {
                reason: error.to_string(),
            },
        )
    })?;
    entries.0.into_iter().map(into_field).collect()
}

fn into_field((key, raw): (String, Raw)) -> Result<Field, FieldFailure> {
    if key.is_empty() || key.chars().count() > MAX_KEY_LENGTH {
        return Err((None, PageErrorKind::FieldKeyLength { key }));
    }
    let texts = match raw {
        Raw::Scalar(text) => vec![text],
        Raw::List(texts) => texts,
        Raw::Null => Vec::new(),
        Raw::Invalid => return Err((None, PageErrorKind::FieldNotScalar { key })),
    };
    if texts.iter().any(|text| text.len() > MAX_VALUE_BYTES) {
        return Err((None, PageErrorKind::FieldValueTooLong { key }));
    }
    let values = texts
        .iter()
        .map(|text| FieldValue::classify(text))
        .collect();
    Ok(Field { key, values })
}

enum Raw {
    Scalar(String),
    List(Vec<String>),
    Null,
    Invalid,
}

struct Entries(Vec<(String, Raw)>);

impl<'de> Deserialize<'de> for Entries {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(EntriesVisitor)
    }
}

struct EntriesVisitor;

impl<'de> Visitor<'de> for EntriesVisitor {
    type Value = Entries;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a mapping of field names to values")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Entries, A::Error> {
        let mut entries = Vec::new();
        while let Some(entry) = map.next_entry::<String, Raw>()? {
            entries.push(entry);
        }
        Ok(Entries(entries))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Entries, E> {
        Ok(Entries(Vec::new()))
    }
}

impl<'de> Deserialize<'de> for Raw {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(RawVisitor)
    }
}

struct RawVisitor;

impl<'de> Visitor<'de> for RawVisitor {
    type Value = Raw;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("a scalar or a list of scalars")
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> Result<Raw, E> {
        Ok(Raw::Scalar(value.to_string()))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> Result<Raw, E> {
        Ok(Raw::Scalar(value.to_string()))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> Result<Raw, E> {
        Ok(Raw::Scalar(value.to_string()))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> Result<Raw, E> {
        Ok(Raw::Scalar(value.to_string()))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<Raw, E> {
        Ok(Raw::Scalar(value.to_owned()))
    }

    fn visit_unit<E: de::Error>(self) -> Result<Raw, E> {
        Ok(Raw::Null)
    }

    fn visit_none<E: de::Error>(self) -> Result<Raw, E> {
        Ok(Raw::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Raw, A::Error> {
        let mut texts = Vec::new();
        let mut valid = true;
        while let Some(item) = seq.next_element::<Raw>()? {
            match item {
                Raw::Scalar(text) => texts.push(text),
                Raw::List(_) | Raw::Null | Raw::Invalid => valid = false,
            }
        }
        Ok(if valid {
            Raw::List(texts)
        } else {
            Raw::Invalid
        })
    }

    fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Raw, A::Error> {
        while map
            .next_entry::<de::IgnoredAny, de::IgnoredAny>()?
            .is_some()
        {}
        Ok(Raw::Invalid)
    }
}
