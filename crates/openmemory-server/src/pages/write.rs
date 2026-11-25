use openmemory_ops::{
    Append, Page, PagePath, Replacement, Source, VersionOp, WriteAction, WriteOutput,
    replace_unique,
};
use sqlx::{PgConnection, PgPool};

use crate::access::Actor;
use crate::pages::commit::{self, NextState};
use crate::pages::content;
use crate::pages::error::PagesError;
use crate::pages::group::{self, GroupJoin};
use crate::pages::guard;
use crate::pages::ids::{GroupId, PageId, VersionNumber};
use crate::pages::locate::{lock_live, lock_live_by_id};
use crate::pages::queries::pages::{self, PageRow};
use crate::pages::quote;
use crate::time_zone::TimeZone;

#[derive(Debug)]
pub(crate) enum Revision {
    Overwrite {
        content: String,
    },
    Edit {
        replace: Option<Replacement>,
        append: Option<Append>,
    },
}

pub(crate) async fn write(
    pool: &PgPool,
    time_zone: &TimeZone,
    actor: &Actor,
    path: &PagePath,
    action: WriteAction,
    task: Option<&str>,
) -> Result<WriteOutput, PagesError> {
    guard::require_write(actor, path)?;
    let task = guard::task(task)?;
    let mut tx = pool.begin().await?;
    let group = group::assign(&mut tx, actor, task, GroupJoin::Continue).await?;
    let written = match action {
        WriteAction::Create { content } => create(&mut tx, actor, group, path, &content).await?,
        WriteAction::Edit { replace, append } => {
            let row = lock_live(&mut tx, path).await?;
            edit(&mut tx, time_zone, actor, group, &row, replace, append).await?
        }
        WriteAction::Overwrite {
            content,
            base_version,
        } => {
            let row = lock_live(&mut tx, path).await?;
            guard::require_version(&row, VersionNumber(base_version))?;
            overwrite(&mut tx, actor, group, &row, &content).await?
        }
    };
    tx.commit().await?;
    Ok(written)
}

pub(crate) async fn write_page(
    pool: &PgPool,
    time_zone: &TimeZone,
    actor: &Actor,
    id: PageId,
    expected: VersionNumber,
    revision: Revision,
    task: Option<&str>,
) -> Result<WriteOutput, PagesError> {
    let task = guard::task(task)?;
    let mut tx = pool.begin().await?;
    let group = group::assign(&mut tx, actor, task, GroupJoin::Continue).await?;
    let row = lock_live_by_id(&mut tx, id).await?;
    guard::require_write_visible(actor, &row.path)?;
    guard::require_version(&row, expected)?;
    let written = match revision {
        Revision::Overwrite { content } => overwrite(&mut tx, actor, group, &row, &content).await?,
        Revision::Edit { replace, append } => {
            edit(&mut tx, time_zone, actor, group, &row, replace, append).await?
        }
    };
    tx.commit().await?;
    Ok(written)
}

async fn create(
    conn: &mut PgConnection,
    actor: &Actor,
    group: GroupId,
    path: &PagePath,
    text: &str,
) -> Result<WriteOutput, PagesError> {
    let prepared = content::prepare(text)?;
    content::reject_credentials(&prepared.text)?;
    if pages::path_in_use(conn, path.as_str(), None).await? {
        return Err(PagesError::PathTaken);
    }
    commit::create_page(conn, actor, group, path.as_str(), &prepared.text).await
}

async fn edit(
    conn: &mut PgConnection,
    time_zone: &TimeZone,
    actor: &Actor,
    group: GroupId,
    row: &PageRow,
    replace: Option<Replacement>,
    append: Option<Append>,
) -> Result<WriteOutput, PagesError> {
    let mut text = row.content.clone();
    if let Some(replacement) = &replace {
        content::reject_credentials(&replacement.new)?;
        text = replace_unique(&text, &replacement.old, &replacement.new)?;
    }
    if let Some(addition) = &append {
        content::reject_credentials(&addition.text)?;
        let source = addition.source.as_deref().unwrap_or(&actor.name);
        content::reject_credentials(source)?;
        let today = time_zone.today(conn).await?;
        let date = addition.date.unwrap_or(today);
        if date > today {
            return Err(PagesError::FutureDate { today });
        }
        let mut page = Page::parse(&text)?;
        page.append_timeline(date, source, &addition.text)?;
        if let Some(entry) = page.timeline().last()
            && let Source::File {
                path,
                page: number,
                quote: Some(quote),
            } = entry.source()
        {
            quote::verify(conn, actor, path, *number, quote).await?;
        }
        text = page.to_text();
    }
    finish(conn, actor, group, row, &text).await
}

async fn overwrite(
    conn: &mut PgConnection,
    actor: &Actor,
    group: GroupId,
    row: &PageRow,
    text: &str,
) -> Result<WriteOutput, PagesError> {
    content::reject_credentials(text)?;
    finish(conn, actor, group, row, text).await
}

async fn finish(
    conn: &mut PgConnection,
    actor: &Actor,
    group: GroupId,
    row: &PageRow,
    text: &str,
) -> Result<WriteOutput, PagesError> {
    let prepared = content::prepare(text)?;
    content::require_append_only(&row.content, &prepared.page)?;
    if prepared.text == row.content {
        return Err(PagesError::NoChange);
    }
    let next = NextState {
        path: &row.path,
        content: &prepared.text,
        is_deleted: false,
        op: VersionOp::Edit,
        reverts_version: None,
    };
    commit::commit(conn, actor, group, row, &next).await
}
