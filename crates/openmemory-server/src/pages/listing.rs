use sqlx::PgPool;
use time::OffsetDateTime;

use crate::access::Actor;
use crate::pages::error::PagesError;
use crate::pages::ids::{PageId, VersionNumber};
use crate::pages::queries::listing;
use crate::pages::scope_filter::ScopeFilter;

#[derive(Debug, Clone)]
pub(crate) struct PageSummary {
    pub(crate) id: PageId,
    pub(crate) path: String,
    pub(crate) version: VersionNumber,
    pub(crate) updated_at: OffsetDateTime,
}

#[derive(Debug, Clone)]
pub(crate) struct PageSource {
    pub(crate) path: String,
    pub(crate) content: String,
}

pub(crate) async fn summaries(
    pool: &PgPool,
    actor: &Actor,
) -> Result<Vec<PageSummary>, PagesError> {
    let mut conn = pool.acquire().await?;
    let rows = listing::list_live(&mut conn, &ScopeFilter::of(&actor.scope)).await?;
    Ok(rows
        .into_iter()
        .map(|row| PageSummary {
            id: row.id,
            path: row.path,
            version: row.current_version,
            updated_at: row.updated_at,
        })
        .collect())
}

pub(crate) async fn sources(pool: &PgPool, actor: &Actor) -> Result<Vec<PageSource>, PagesError> {
    let mut conn = pool.acquire().await?;
    let rows = listing::list_live_content(&mut conn, &ScopeFilter::of(&actor.scope)).await?;
    Ok(rows
        .into_iter()
        .map(|row| PageSource {
            path: row.path,
            content: row.content,
        })
        .collect())
}

pub(crate) async fn ids_for_paths(
    pool: &PgPool,
    paths: &[String],
) -> Result<Vec<(String, PageId)>, PagesError> {
    let mut conn = pool.acquire().await?;
    Ok(listing::ids_for_paths(&mut conn, paths).await?)
}
