use rmcp::ErrorData;
use rmcp::model::{CallToolResult, ContentBlock};
use serde::Serialize;

use crate::mcp::error::ToolError;

pub(super) fn respond<T: Serialize>(
    outcome: Result<T, ToolError>,
) -> Result<CallToolResult, ErrorData> {
    match outcome {
        Ok(value) => {
            let text = serde_json::to_string(&value).map_err(|error| {
                tracing::error!(%error, "serialize tool result");
                ErrorData::internal_error("internal error", None)
            })?;
            Ok(CallToolResult::success(vec![ContentBlock::text(text)]))
        }
        Err(ToolError::Rejected(message)) => {
            Ok(CallToolResult::error(vec![ContentBlock::text(message)]))
        }
        Err(ToolError::Internal(source)) => {
            tracing::error!(error = %source, "tool failed");
            Err(ErrorData::internal_error("internal error", None))
        }
    }
}
