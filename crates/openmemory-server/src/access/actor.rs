use serde::{Deserialize, Serialize};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type, Serialize, Deserialize, utoipa::ToSchema,
)]
#[sqlx(transparent)]
#[serde(transparent)]
pub(crate) struct UserId(pub(crate) i64);

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, sqlx::Type, Serialize, Deserialize, utoipa::ToSchema,
)]
#[sqlx(transparent)]
#[serde(transparent)]
pub(crate) struct ConnectionId(pub(crate) i64);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Actor {
    pub(crate) user_id: UserId,
    pub(crate) connection_id: Option<ConnectionId>,
    pub(crate) name: String,
    pub(crate) scope: crate::access::Scope,
}
