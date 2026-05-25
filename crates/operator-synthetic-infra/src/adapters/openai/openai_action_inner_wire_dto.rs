use serde::{Deserialize, Serialize};

use crate::adapters::openai::openai_arguments_wire_dto::OpenAIArgumentsWireDto;

/// Adapter-specific inner action shape for `OpenAI` structured outputs.
///
/// `OpenAI` strict schemas require every property to be present. Non-applicable
/// fields are represented as `null` and rejected post-hoc when inconsistent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenAIActionInnerWireDto {
    pub kind: String,
    pub tool: Option<String>,
    pub arguments: Option<OpenAIArgumentsWireDto>,
    pub reason: Option<String>,
    pub answer: Option<String>,
    #[serde(default)]
    pub evidence: Vec<String>,
    pub target_model: Option<String>,
}
