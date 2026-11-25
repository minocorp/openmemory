use openmemory_ops::{Page, VersionOp, WriteOutput, link_targets};
use sqlx::PgConnection;

use crate::access::Actor;
use crate::pages::error::PagesError;
use crate::pages::ids::{GroupId, PageId, VersionNumber};
use crate::pages::queries::index::{self, FieldRows};
use crate::pages::queries::pages::{self, PageRow};
use crate::pages::queries::versions::{self, NewVersion};

#[derive(Debug)]
pub(super) struct NextState<'a> {
    pub(super) path: &'a str,
    pub(super) content: &'a str,
    pub(super) is_deleted: bool,
    pub(super) op: VersionOp,
    pub(super) reverts_version: Option<VersionNumber>,
}

pub(super) async fn create_page(
    conn: &mut PgConnection,
    actor: &Actor,
    group: GroupId,
    path: &str,
    content: &str,
) -> Result<WriteOutput, PagesError> {
    let id = pages::insert(conn, path, content)
        .await
        .map_err(map_path_conflict)?;
    let version = NewVersion {
        page_id: id,
        n: VersionNumber(1),
        content,
        path,
        op: VersionOp::Create.as_str(),
        reverts_version: None,
        group_id: group,
    };
    versions::insert(conn, actor, &version).await?;
    reindex(conn, id, content, false).await?;
    Ok(WriteOutput {
        path: path.to_owned(),
        version: 1,
    })
}

pub(super) async fn commit(
    conn: &mut PgConnection,
    actor: &Actor,
    group: GroupId,
    page: &PageRow,
    next: &NextState<'_>,
) -> Result<WriteOutput, PagesError> {
    let n = VersionNumber(page.current_version.0 + 1);
    let swapped = pages::update_if_current(
        conn,
        page.id,
        page.current_version,
        n,
        next.path,
        next.content,
        next.is_deleted,
    )
    .await
    .map_err(map_path_conflict)?;
    if !swapped {
        return Err(PagesError::Conflict {
            current_version: current_version(conn, page.id).await?,
        });
    }
    let version = NewVersion {
        page_id: page.id,
        n,
        content: next.content,
        path: next.path,
        op: next.op.as_str(),
        reverts_version: next.reverts_version,
        group_id: group,
    };
    versions::insert(conn, actor, &version).await?;
    reindex(conn, page.id, next.content, next.is_deleted).await?;
    Ok(WriteOutput {
        path: next.path.to_owned(),
        version: n.0,
    })
}

async fn current_version(conn: &mut PgConnection, id: PageId) -> Result<VersionNumber, PagesError> {
    let rows = pages::lock_many(conn, &[id.0]).await?;
    Ok(rows
        .first()
        .map_or(VersionNumber(0), |row| row.current_version))
}

fn map_path_conflict(error: sqlx::Error) -> PagesError {
    if PagesError::is_unique_violation(&error) {
        PagesError::PathTaken
    } else {
        PagesError::Database(error)
    }
}

async fn reindex(
    conn: &mut PgConnection,
    id: PageId,
    content: &str,
    is_deleted: bool,
) -> Result<(), PagesError> {
    if is_deleted {
        index::replace_fields(conn, id, &FieldRows::default()).await?;
        index::replace_links(conn, id, &[], &[]).await?;
        return Ok(());
    }
    let page = Page::parse(content)?;
    index::replace_fields(conn, id, &field_rows(&page)).await?;
    let paths: Vec<String> = link_targets(content)
        .iter()
        .map(|path| path.as_str().to_owned())
        .collect();
    let to_pages = index::resolve_page_targets(conn, &paths, id).await?;
    let to_files = index::resolve_file_targets(conn, &paths).await?;
    index::replace_links(conn, id, &to_pages, &to_files).await?;
    Ok(())
}

fn field_rows(page: &Page) -> FieldRows {
    let mut rows = FieldRows::default();
    for field in page.fields() {
        for (ord, value) in field.values().iter().enumerate() {
            rows.keys.push(field.key().to_owned());
            rows.ords.push(i32::try_from(ord).unwrap_or(i32::MAX));
            rows.texts.push(value.text().to_owned());
            rows.numbers
                .push(value.number().unwrap_or_default().to_owned());
            rows.dates.push(
                value
                    .date()
                    .map_or_else(String::new, openmemory_ops::format_iso_date),
            );
        }
    }
    rows
}
