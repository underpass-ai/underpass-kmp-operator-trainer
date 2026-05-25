use serde::{Deserialize, Serialize};
use serde_json::Value;

/// `OpenAI` structured-output arguments payload. The semantic tool pairing is
/// checked by `OpenAIActionWireDtoMapper`, not by the provider schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OpenAIArgumentsWireDto {
    pub value: Value,
}

impl OpenAIArgumentsWireDto {
    #[must_use]
    pub fn new(value: Value) -> Self {
        Self { value }
    }

    #[must_use]
    pub fn into_value(self) -> Value {
        self.value
    }
}
