use memory_ops::{PagePath, WriteAction, WriteInput, WriteOutput};

use crate::access::Actor;
use crate::mcp::error::ToolError;
use crate::pages::error::PagesError;
use crate::pages::write;
use crate::state::AppState;

pub(in crate::mcp) async fn write(
    state: &AppState,
    actor: &Actor,
    input: WriteInput,
) -> Result<WriteOutput, ToolError> {
    let action = input.action()?;
    let path = PagePath::parse(&input.path)?;
    let creating = matches!(action, WriteAction::Create { .. });
    match write::write(
        state.pool(),
        state.time_zone(),
        actor,
        &path,
        action,
        input.task.as_deref(),
    )
    .await
    {
        Err(PagesError::PathTaken) if creating => Err(ToolError::rejected(
            "A page or file already exists at this path, so `content` alone cannot create it. `read` the page; to replace its whole content pass `content` with its `base_version`, or edit it with `replace_old`, `replace_new` and `append_text`.",
        )),
        written => Ok(written?),
    }
}
