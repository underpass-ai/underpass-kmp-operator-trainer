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
    pub max_completion_tokens: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_format: Option<Value>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn serializes_explicit_max_completion_tokens() {
        let request = OpenAiChatCompletionRequestDto {
            model: "gpt-4o-mini".to_string(),
            messages: vec![OpenAiChatCompletionRequestMessageDto {
                role: "user".to_string(),
                content: "Return one action.".to_string(),
            }],
            temperature: 0.0,
            max_tokens: None,
            max_completion_tokens: Some(4096),
            response_format: None,
        };

        let serialized = serde_json::to_value(request).expect("request serializes");

        assert_eq!(serialized["max_completion_tokens"], json!(4096));
        assert!(serialized.get("max_tokens").is_none());
    }
}
