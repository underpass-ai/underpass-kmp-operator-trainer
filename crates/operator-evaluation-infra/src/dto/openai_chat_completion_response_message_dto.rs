//! The assistant message inside a chat-completions choice. The
//! baseliner only consumes `content`; other fields (`role`,
//! `refusal`, tool calls) are ignored so the same DTO works across
//! `OpenAI`, vLLM, and Anthropic OpenAI-compat responses.

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiChatCompletionResponseMessageDto {
    pub content: String,
}
