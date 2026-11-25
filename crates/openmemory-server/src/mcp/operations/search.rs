use openmemory_ops::{Envelope, HitKind, Origin, SearchHit, SearchInput, SearchOutput};

use crate::access::Actor;
use crate::mcp::error::ToolError;
use crate::search::{self, Match};
use crate::state::AppState;

pub(in crate::mcp) async fn search(
    state: &AppState,
    actor: &Actor,
    input: SearchInput,
) -> Result<Envelope<SearchOutput>, ToolError> {
    let output =
        search::search(state.pool(), state.time_zone(), actor, &input, Match::Text).await?;
    let origin = Origin::merged(output.hits.iter().map(origin_of));
    Ok(Envelope::new(origin, output))
}

fn origin_of(hit: &SearchHit) -> Origin {
    match hit.kind {
        HitKind::Page => Origin::of_author(&hit.author),
        HitKind::File => Origin::Extracted,
    }
}
