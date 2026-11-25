use openmemory_ops::{Envelope, HistoryInput, HistoryOutput, Origin, PagePath, parse_iso_date};

use crate::access::Actor;
use crate::mcp::error::ToolError;
use crate::pages::history::{self, HistoryQuery};
use crate::state::AppState;

pub(in crate::mcp) async fn history(
    state: &AppState,
    actor: &Actor,
    input: HistoryInput,
) -> Result<Envelope<HistoryOutput>, ToolError> {
    let path = input.path.as_deref().map(PagePath::parse).transpose()?;
    let since = input
        .since
        .as_deref()
        .map(|text| {
            parse_iso_date(text).ok_or_else(|| {
                ToolError::rejected("Invalid `since`: use an ISO date such as `2026-11-01`.")
            })
        })
        .transpose()?;
    let query = HistoryQuery {
        path: path.as_ref(),
        by: input.by.as_deref(),
        since,
        limit: input.limit,
    };
    let output = history::history(state.pool(), actor, &query).await?;
    let origin = Origin::merged(
        output
            .versions
            .iter()
            .map(|version| Origin::of_author(&version.author)),
    );
    Ok(Envelope::new(origin, output))
}
