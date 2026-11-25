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
    Serialize,
    Deserialize,
    sqlx::Type,
    utoipa::ToSchema,
)]
#[sqlx(transparent)]
#[serde(transparent)]
pub(crate) struct PageId(pub(crate) i64);

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    sqlx::Type,
    utoipa::ToSchema,
)]
#[sqlx(transparent)]
#[serde(transparent)]
pub(crate) struct GroupId(pub(crate) i64);

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    sqlx::Type,
    utoipa::ToSchema,
)]
#[sqlx(transparent)]
#[serde(transparent)]
pub(crate) struct Seq(pub(crate) i64);

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    sqlx::Type,
    utoipa::ToSchema,
)]
#[sqlx(transparent)]
#[serde(transparent)]
pub(crate) struct VersionNumber(pub(crate) i32);
