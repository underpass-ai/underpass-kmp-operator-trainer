//! Error envelope returned by OpenAI-compatible endpoints on a
//! non-2xx response: `{"error": {"message": "...", "code": "..."}}`.

use serde::Deserialize;

use crate::dto::openai_chat_completion_error_dto::OpenAiChatCompletionErrorDto;

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiChatCompletionErrorEnvelopeDto {
    pub error: OpenAiChatCompletionErrorDto,
}
