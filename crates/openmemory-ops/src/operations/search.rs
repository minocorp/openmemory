use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[schemars(transform = crate::schema::omit_null)]
pub struct SearchInput {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Words to find. A page or file matches when every word occurs in its path or text, ignoring case. Korean and other languages work. Leave out to filter by `where` only."
    )]
    pub text: Option<String>,
    #[serde(default, rename = "where", skip_serializing_if = "Vec::is_empty")]
    #[schemars(
        description = "Field conditions that must all hold, one per list entry: `key=v`, `key!=v` (also matches pages without the key), `key<v`, `key>v`, `key~text`, `key:exists`. ISO dates compare as dates, numbers as numbers, anything else as text. Relative dates: `today`, `+90d`, `-30d`. Files have no fields, so any condition leaves them out. Example: [\"status=active\", \"renewal<+90d\"]."
    )]
    pub conditions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Only search this folder and below, for example `customers`. Pass a `folders` entry of a previous result to see all of its matches."
    )]
    pub folder: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "Maximum pages and files in `hits`, and maximum `folders`, 1 to 100. Default 20."
    )]
    pub limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(
        description = "The `next_cursor` of a previous result, to get the next page of `hits`."
    )]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchOutput {
    pub hits: Vec<SearchHit>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub folders: Vec<FolderHits>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_hits: Option<NoHits>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FolderHits {
    pub path: String,
    pub matches: u32,
    pub hits: Vec<SearchHit>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HitKind {
    Page,
    File,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SearchHit {
    pub kind: HitKind,
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub page: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line_number: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub line: Option<String>,
    pub author: String,
    pub at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NoHits {
    pub query: String,
    pub fields_in_scope: Vec<String>,
    pub folders_out_of_scope: u32,
}
