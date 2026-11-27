use serde::{Deserialize, Serialize};

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    sqlx::Type,
    Serialize,
    Deserialize,
    utoipa::ToSchema,
)]
#[sqlx(transparent)]
#[serde(transparent)]
pub(crate) struct FileId(pub(crate) i64);
