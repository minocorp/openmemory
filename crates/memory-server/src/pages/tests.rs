mod access;
mod by_id;
mod changes;
mod groups;
mod history;
mod indexing;
mod purge;
mod relocation;
mod reverts;
mod writes;

use memory_ops::{Append, Replacement, WriteAction, WriteOutput};
use sqlx::PgPool;

use crate::access::Actor;
use crate::pages::error::PagesError;
use crate::pages::write::write;
use crate::test_support::path;
use crate::time_zone::TimeZone;

pub(super) const PAGE: &str = "---\nstatus: active\nfee: 500000\n---\nPartner. 500,000 KRW.\n\n## Timeline\n2026-03-02 | call — signed\n";

pub(super) async fn edit(
    pool: &PgPool,
    actor: &Actor,
    page_path: &str,
    replace: Option<(&str, &str)>,
    append: Option<&str>,
) -> Result<WriteOutput, PagesError> {
    let action = WriteAction::Edit {
        replace: replace.map(|(old, new)| Replacement {
            old: old.to_owned(),
            new: new.to_owned(),
        }),
        append: append.map(|text| Append {
            text: text.to_owned(),
            source: None,
            date: None,
        }),
    };
    write(
        pool,
        &TimeZone::utc(),
        actor,
        &path(page_path),
        action,
        None,
    )
    .await
}

pub(super) async fn overwrite(
    pool: &PgPool,
    actor: &Actor,
    page_path: &str,
    content: &str,
    base_version: i32,
) -> Result<WriteOutput, PagesError> {
    let action = WriteAction::Overwrite {
        content: content.to_owned(),
        base_version,
    };
    write(
        pool,
        &TimeZone::utc(),
        actor,
        &path(page_path),
        action,
        None,
    )
    .await
}
