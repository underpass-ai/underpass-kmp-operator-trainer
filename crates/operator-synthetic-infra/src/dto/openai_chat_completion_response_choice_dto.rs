use serde::{Deserialize, Serialize};

use crate::dto::openai_chat_completion_response_message_dto::OpenAiChatCompletionResponseMessageDto;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenAiChatCompletionResponseChoiceDto {
    pub message: OpenAiChatCompletionResponseMessageDto,
    #[serde(default)]
    pub finish_reason: Option<String>,
}
