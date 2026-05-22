use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::dto::openai_chat_completion_request_message_dto::OpenAiChatCompletionRequestMessageDto;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OpenAiChatCompletionRequestDto {
    pub model: String,
    pub messages: Vec<OpenAiChatCompletionRequestMessageDto>,
    pub temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<Value>,
}
