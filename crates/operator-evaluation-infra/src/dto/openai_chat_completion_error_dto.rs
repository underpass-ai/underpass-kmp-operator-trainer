//! Inner error body returned inside an OpenAI-compat error envelope.
//! `code` and `kind` (`type` in the wire format) are both optional —
//! `OpenAI` populates `code`, Anthropic-via-compat populates `type`,
//! vLLM may populate neither.

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiChatCompletionErrorDto {
    pub message: String,
    #[serde(default)]
    pub code: Option<String>,
    #[serde(default, rename = "type")]
    pub kind: Option<String>,
}
