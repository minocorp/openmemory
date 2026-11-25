use std::fmt;

use time::{Date, Duration};

use crate::iso_date::format_iso_date;

use crate::iso_date::parse_iso_date;
use crate::page::is_number;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WhereClause {
    conditions: Vec<Condition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Condition {
    key: String,
    test: Test,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Test {
    Exists,
    Compare(Comparison, Operand),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Comparison {
    Equal,
    NotEqual,
    Less,
    Greater,
    Contains,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Operand {
    Date(Date),
    DaysFromToday(i64),
    Number(String),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Date(Date),
    Number(String),
    Text(String),
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[error("condition {index}: {kind}")]
pub struct WhereError {
    index: usize,
    kind: WhereErrorKind,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum WhereErrorKind {
    #[error("field name is missing")]
    MissingKey,
    #[error("operator is missing or unknown")]
    UnknownOperator,
    #[error("value is missing")]
    MissingValue,
    #[error("date is out of range")]
    DateOutOfRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCondition {
    key: String,
    test: ResolvedTest,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedTest {
    Exists,
    Compare(Comparison, Value),
}

impl ResolvedCondition {
    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn test(&self) -> &ResolvedTest {
        &self.test
    }
}

impl fmt::Display for ResolvedCondition {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ResolvedTest::Compare(comparison, value) = &self.test else {
            return write!(formatter, "{} exists", self.key);
        };
        let symbol = match comparison {
            Comparison::Equal => "=",
            Comparison::NotEqual => "!=",
            Comparison::Less => "<",
            Comparison::Greater => ">",
            Comparison::Contains => "contains",
        };
        match value {
            Value::Date(date) => write!(
                formatter,
                "{} {symbol} date {}",
                self.key,
                format_iso_date(*date)
            ),
            Value::Number(number) => write!(formatter, "{} {symbol} number {number}", self.key),
            Value::Text(text) => write!(formatter, "{} {symbol} text \"{text}\"", self.key),
        }
    }
}

impl WhereClause {
    pub fn parse(conditions: &[String]) -> Result<Self, WhereError> {
        conditions
            .iter()
            .enumerate()
            .map(|(index, condition)| {
                Condition::parse(condition).map_err(|kind| WhereError { index, kind })
            })
            .collect::<Result<_, _>>()
            .map(|conditions| Self { conditions })
    }

    pub fn conditions(&self) -> &[Condition] {
        &self.conditions
    }

    pub fn is_empty(&self) -> bool {
        self.conditions.is_empty()
    }

    pub fn resolve(&self, today: Date) -> Result<Vec<ResolvedCondition>, WhereError> {
        self.conditions
            .iter()
            .enumerate()
            .map(|(index, condition)| {
                let test = match &condition.test {
                    Test::Exists => ResolvedTest::Exists,
                    Test::Compare(comparison, operand) => ResolvedTest::Compare(
                        *comparison,
                        operand
                            .resolve(today)
                            .map_err(|kind| WhereError { index, kind })?,
                    ),
                };
                Ok(ResolvedCondition {
                    key: condition.key.clone(),
                    test,
                })
            })
            .collect()
    }
}

impl WhereError {
    pub fn index(&self) -> usize {
        self.index
    }

    pub fn kind(&self) -> &WhereErrorKind {
        &self.kind
    }
}

impl Condition {
    fn parse(input: &str) -> Result<Self, WhereErrorKind> {
        let operator_start = input
            .find(['=', '!', '<', '>', '~', ':'])
            .ok_or(WhereErrorKind::UnknownOperator)?;
        let key = input.get(..operator_start).unwrap_or_default().trim();
        if key.is_empty() {
            return Err(WhereErrorKind::MissingKey);
        }
        let rest = input.get(operator_start..).unwrap_or_default();
        let test = if rest == ":exists" {
            Test::Exists
        } else {
            let (comparison, value) = split_operator(rest)?;
            let value = value.trim();
            if value.is_empty() {
                return Err(WhereErrorKind::MissingValue);
            }
            Test::Compare(comparison, Operand::parse(comparison, value))
        };
        Ok(Self {
            key: key.to_owned(),
            test,
        })
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn test(&self) -> &Test {
        &self.test
    }
}

impl Operand {
    fn parse(comparison: Comparison, value: &str) -> Self {
        if comparison == Comparison::Contains {
            return Self::Text(value.to_owned());
        }
        if value == "today" {
            return Self::DaysFromToday(0);
        }
        if let Some(days) = relative_days(value) {
            return Self::DaysFromToday(days);
        }
        if let Some(date) = parse_iso_date(value) {
            return Self::Date(date);
        }
        if is_number(value) {
            return Self::Number(value.to_owned());
        }
        Self::Text(value.to_owned())
    }

    pub fn resolve(&self, today: Date) -> Result<Value, WhereErrorKind> {
        match self {
            Self::Date(date) => Ok(Value::Date(*date)),
            Self::DaysFromToday(days) => i32::try_from(*days)
                .ok()
                .and_then(|days| today.checked_add(Duration::days(i64::from(days))))
                .map(Value::Date)
                .ok_or(WhereErrorKind::DateOutOfRange),
            Self::Number(number) => Ok(Value::Number(number.clone())),
            Self::Text(text) => Ok(Value::Text(text.clone())),
        }
    }
}

fn split_operator(rest: &str) -> Result<(Comparison, &str), WhereErrorKind> {
    let operators = [
        ("!=", Comparison::NotEqual),
        ("=", Comparison::Equal),
        ("<", Comparison::Less),
        (">", Comparison::Greater),
        ("~", Comparison::Contains),
    ];
    operators
        .into_iter()
        .find_map(|(symbol, comparison)| rest.strip_prefix(symbol).map(|value| (comparison, value)))
        .ok_or(WhereErrorKind::UnknownOperator)
}

fn relative_days(value: &str) -> Option<i64> {
    let digits = value.strip_suffix('d')?;
    let magnitude = digits
        .strip_prefix('+')
        .or_else(|| digits.strip_prefix('-'))?;
    if magnitude.is_empty() || !magnitude.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    let days: i64 = magnitude.parse().ok()?;
    Some(if digits.starts_with('-') { -days } else { days })
}

#[cfg(test)]
mod tests;
