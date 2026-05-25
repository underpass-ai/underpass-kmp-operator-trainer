use serde::{Deserialize, Serialize};

use crate::adapters::openai::openai_action_inner_wire_dto::OpenAIActionInnerWireDto;

/// Adapter-specific DTO for `OpenAI` structured output wire shape.
///
/// This is deliberately separate from `OperatorActionDto`: `OpenAI` strict
/// schemas require every property to be required, while the domain contract is
/// an externally tagged enum.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenAIActionWireDto {
    pub action: OpenAIActionInnerWireDto,
}
