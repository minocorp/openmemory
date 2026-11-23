use memory_ops::{PagePath, RevertInput, RevertOutput, RevertTarget};

use crate::access::Actor;
use crate::mcp::error::ToolError;
use crate::pages::ids::{GroupId, VersionNumber};
use crate::pages::revert;
use crate::state::AppState;

pub(in crate::mcp) async fn revert(
    state: &AppState,
    actor: &Actor,
    input: RevertInput,
) -> Result<RevertOutput, ToolError> {
    let task = input.task.as_deref();
    let output = match input.target()? {
        RevertTarget::Version { path, version } => {
            let path = PagePath::parse(&path)?;
            revert::revert_version(state.pool(), actor, &path, VersionNumber(version), task).await
        }
        RevertTarget::Group { group } => {
            revert::revert_group(state.pool(), actor, GroupId(group), task).await
        }
    };
    Ok(output?)
}
