use openmemory_ops::{Envelope, Origin, SearchInput, SearchOutput};

use crate::access::Actor;
use crate::mcp::error::ToolError;
use crate::search::{self, Document, FoundHit};
use crate::state::AppState;

pub(in crate::mcp) async fn search(
    state: &AppState,
    actor: &Actor,
    input: SearchInput,
) -> Result<Envelope<SearchOutput>, ToolError> {
    let found = search::search(state.pool(), state.time_zone(), actor, &input).await?;
    let shown = found
        .hits
        .iter()
        .chain(found.folders.iter().flat_map(|folder| folder.hits.iter()));
    let origin = Origin::merged(shown.map(origin_of));
    Ok(Envelope::new(origin, found.into()))
}

fn origin_of(found: &FoundHit) -> Origin {
    match found.document {
        Document::Page(_) => Origin::of_author(&found.hit.author),
        Document::File(_) => Origin::Extracted,
    }
}
