use memory_ops::{DeleteInput, MoveInput, MoveOutput, PagePath, WriteOutput};

use crate::access::Actor;
use crate::mcp::error::ToolError;
use crate::pages::{delete, relocate};
use crate::state::AppState;

pub(in crate::mcp) async fn move_page(
    state: &AppState,
    actor: &Actor,
    input: MoveInput,
) -> Result<MoveOutput, ToolError> {
    let from = PagePath::parse(&input.from)?;
    let to = PagePath::parse(&input.to)?;
    Ok(relocate::move_page(state.pool(), actor, &from, &to, input.task.as_deref()).await?)
}

pub(in crate::mcp) async fn delete(
    state: &AppState,
    actor: &Actor,
    input: DeleteInput,
) -> Result<WriteOutput, ToolError> {
    let path = PagePath::parse(&input.path)?;
    Ok(delete::delete(state.pool(), actor, &path, input.task.as_deref()).await?)
}
