//! Wire shape returned by `/chat/completions` on any OpenAI-compatible
//! endpoint. Captures only the fields the baseliner consumes:
//! `choices[0].message.content`. Everything else is ignored.

use serde::Deserialize;

use crate::dto::openai_chat_completion_response_choice_dto::OpenAiChatCompletionResponseChoiceDto;

#[derive(Debug, Clone, Deserialize)]
pub struct OpenAiChatCompletionResponseDto {
    pub choices: Vec<OpenAiChatCompletionResponseChoiceDto>,
}
