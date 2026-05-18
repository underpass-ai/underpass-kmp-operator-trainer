use serde::{Deserialize, Serialize};

use crate::escalate_action_dto::EscalateActionDto;
use crate::stop_action_dto::StopActionDto;
use crate::tool_call_action_dto::ToolCallActionDto;

/// Externally tagged enum: `{"kind": "tool_call", "tool": "...",
/// "arguments": {...}}` etc. The `kind` discriminator is the field name in
/// the wire format.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OperatorActionDto {
    ToolCall(ToolCallActionDto),
    Stop(StopActionDto),
    Escalate(EscalateActionDto),
}
