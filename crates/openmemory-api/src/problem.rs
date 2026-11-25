use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::ToSchema;

pub const TYPE_PREFIX: &str = "urn:openmemory:problem:";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Problem {
    #[serde(rename = "type")]
    pub kind: String,
    pub title: String,
    pub status: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(flatten)]
    #[schema(value_type = Object, additional_properties)]
    pub extensions: Map<String, Value>,
}

impl Problem {
    pub fn new(code: &str, status: u16) -> Self {
        Self {
            kind: format!("{TYPE_PREFIX}{code}"),
            title: title_of(code),
            status,
            detail: None,
            extensions: Map::new(),
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }

    pub fn with(mut self, name: &str, value: impl Into<Value>) -> Self {
        self.extensions.insert(name.to_owned(), value.into());
        self
    }

    pub fn code(&self) -> &str {
        self.kind.strip_prefix(TYPE_PREFIX).unwrap_or(&self.kind)
    }

    pub fn extension(&self, name: &str) -> Option<&Value> {
        self.extensions.get(name)
    }
}

fn title_of(code: &str) -> String {
    let mut words = code.split('_');
    let first = words.next().unwrap_or_default();
    let mut title = String::new();
    let mut letters = first.chars();
    if let Some(initial) = letters.next() {
        title.extend(initial.to_uppercase());
        title.push_str(letters.as_str());
    }
    for word in words {
        title.push(' ');
        title.push_str(word);
    }
    title
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_problem_serializes_with_the_standard_members_and_flat_extensions() {
        let problem = Problem::new("path_taken", 409)
            .with_detail("a page already holds the path")
            .with("field", "path");
        let value = serde_json::to_value(&problem).unwrap();
        assert_eq!(value["type"], "urn:openmemory:problem:path_taken");
        assert_eq!(value["title"], "Path taken");
        assert_eq!(value["status"], 409);
        assert_eq!(value["detail"], "a page already holds the path");
        assert_eq!(value["field"], "path");
    }

    #[test]
    fn a_problem_round_trips_and_exposes_its_code() {
        let problem = Problem::new("not_found", 404).with("pattern", "aws");
        let text = serde_json::to_string(&problem).unwrap();
        let parsed: Problem = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed, problem);
        assert_eq!(parsed.code(), "not_found");
        assert_eq!(
            parsed.extension("pattern").and_then(Value::as_str),
            Some("aws")
        );
    }
}
