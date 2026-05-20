//! One element of `choices[]` in an OpenAI-compat chat-completions
//! response. Only the assistant `message` is consumed.

use serde::Deserialize;

use crate::dto::openai_chat_completion_response_message_dto::OpenAiChatCompletionResponseMessageDto;

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiChatCompletionResponseChoiceDto {
    pub message: OpenAiChatCompletionResponseMessageDto,
}
