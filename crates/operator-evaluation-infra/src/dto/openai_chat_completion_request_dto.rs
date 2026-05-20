//! Wire shape posted to `/chat/completions` on any OpenAI-compatible
//! endpoint (`OpenAI`, vLLM, Anthropic via its OpenAI-compat surface).
//! `temperature` is sent on every request so callers cannot
//! accidentally inherit the server-side default for a deterministic
//! baseline run.

use serde::Serialize;

use crate::dto::openai_chat_completion_request_message_dto::OpenAiChatCompletionRequestMessageDto;

#[derive(Debug, Clone, Serialize)]
pub struct OpenAiChatCompletionRequestDto {
    pub model: String,
    pub messages: Vec<OpenAiChatCompletionRequestMessageDto>,
    pub temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}
