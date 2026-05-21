use serde::{Deserialize, Serialize};

use crate::dto::openai_chat_completion_response_choice_dto::OpenAiChatCompletionResponseChoiceDto;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OpenAiChatCompletionResponseDto {
    pub choices: Vec<OpenAiChatCompletionResponseChoiceDto>,
}
