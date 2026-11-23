mod account;
mod bootstrap;
pub(crate) mod ceremony;
pub(crate) mod confirmation;
mod csrf;
mod error;
mod ids;
mod invites;
mod login;
mod passkeys;
mod queries;
mod recovery;
mod session;
mod signup;
pub(crate) mod token;
mod users;

use axum::Router;
use utoipa::OpenApi;

use crate::state::AppState;

pub(crate) use bootstrap::{Bootstrap, arm_if_no_users};
pub(crate) use csrf::SameOrigin;
pub(crate) use error::{AuthError, violated_constraint};
pub(crate) use passkeys::Passkeys;
pub(crate) use session::{BinarySession, WebSession};

pub(crate) fn router() -> Router<AppState> {
    Router::new()
        .merge(signup::router())
        .merge(login::router())
        .merge(account::router())
        .merge(confirmation::router())
        .merge(invites::router())
        .merge(users::router())
}

pub(crate) fn openapi() -> utoipa::openapi::OpenApi {
    let mut document = signup::Doc::openapi();
    document.merge(login::Doc::openapi());
    document.merge(account::Doc::openapi());
    document.merge(confirmation::Doc::openapi());
    document.merge(invites::Doc::openapi());
    document.merge(users::Doc::openapi());
    document
}
