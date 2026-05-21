use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenAiChatCompletionRequestMessageDto {
    pub role: String,
    pub content: String,
}
