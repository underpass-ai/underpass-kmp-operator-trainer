use serde::{Deserialize, Serialize};

use crate::dto::openai_chat_completion_error_dto::OpenAiChatCompletionErrorDto;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenAiChatCompletionErrorEnvelopeDto {
    pub error: OpenAiChatCompletionErrorDto,
}
