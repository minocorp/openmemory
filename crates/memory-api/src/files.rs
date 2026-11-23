use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use utoipa::ToSchema;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct FileView {
    pub id: i64,
    pub path: String,
    pub mime: String,
    pub size: i64,
    pub page_count: i64,
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
pub struct FileText {
    pub path: String,
    pub page: u32,
    pub page_count: u32,
    pub text: String,
}
