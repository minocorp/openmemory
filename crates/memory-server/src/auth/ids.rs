use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type, Serialize, Deserialize, utoipa::ToSchema,
)]
#[sqlx(transparent)]
#[serde(transparent)]
pub(crate) struct PasskeyId(pub(crate) i64);

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type, Serialize, Deserialize, utoipa::ToSchema,
)]
#[sqlx(transparent)]
#[serde(transparent)]
pub(crate) struct InviteId(pub(crate) i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type)]
#[sqlx(transparent)]
pub(crate) struct SessionId(pub(crate) i64);
