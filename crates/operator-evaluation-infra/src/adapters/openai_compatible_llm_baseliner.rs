//! `LlmBaseliner` adapter that speaks the `OpenAI` chat-completions
//! wire format. This single adapter covers every backend our
//! pipeline targets today, because they all expose the same
//! contract:
//!
//! - vLLM (`llm.underpassai.com` via the kernel mTLS ingress) —
//!   serves models with an `--api-key` flag (optional bearer).
//! - `OpenAI` (`api.openai.com/v1`) — bearer token required.
//! - Anthropic (`api.anthropic.com/v1`) — bearer token required;
//!   Anthropic ships an OpenAI-compatible chat-completions endpoint
//!   that takes the same `{model, messages}` body.
//!
//! Callers pick the variant by setting `api_base`, `model`, and an
//! optional `api_key`. The adapter never special-cases provider names
//! — if a future endpoint speaks the same wire format it works
//! without code changes.
//!
//! `temperature` defaults to `0.0` so a baseline run is reproducible.

use std::time::Duration;

use operator_evaluation_application::error::llm_baseliner_error::LlmBaselinerError;
use operator_evaluation_application::ports::chat_message::ChatMessage;
use operator_evaluation_application::ports::llm_baseliner::LlmBaseliner;
use reqwest::blocking::Client;
use reqwest::header::AUTHORIZATION;

use crate::dto::openai_chat_completion_error_envelope_dto::OpenAiChatCompletionErrorEnvelopeDto;
use crate::dto::openai_chat_completion_request_dto::OpenAiChatCompletionRequestDto;
use crate::dto::openai_chat_completion_request_message_dto::OpenAiChatCompletionRequestMessageDto;
use crate::dto::openai_chat_completion_response_dto::OpenAiChatCompletionResponseDto;

const ADAPTER: &str = "openai_compatible_llm_baseliner";
const DEFAULT_TIMEOUT_SECS: u64 = 60;
const DEFAULT_TEMPERATURE: f32 = 0.0;

#[derive(Debug)]
pub struct OpenAiCompatibleLlmBaseliner {
    api_base: String,
    api_key: Option<String>,
    model: String,
    temperature: f32,
    max_tokens: Option<u32>,
    client: Client,
}

impl OpenAiCompatibleLlmBaseliner {
    /// Build an adapter with a default `reqwest::blocking::Client`
    /// (60-second timeout). For mTLS or custom roots, use
    /// [`Self::with_client`] and pass a caller-built client.
    pub fn new(
        api_base: impl Into<String>,
        api_key: Option<String>,
        model: impl Into<String>,
    ) -> Result<Self, LlmBaselinerError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .map_err(|err| LlmBaselinerError::Transport {
                adapter: ADAPTER,
                message: format!("failed to build reqwest client: {err}"),
            })?;
        Ok(Self::with_client(api_base, api_key, model, client))
    }

    pub fn with_client(
        api_base: impl Into<String>,
        api_key: Option<String>,
        model: impl Into<String>,
        client: Client,
    ) -> Self {
        Self {
            api_base: api_base.into(),
            api_key,
            model: model.into(),
            temperature: DEFAULT_TEMPERATURE,
            max_tokens: None,
            client,
        }
    }

    #[must_use]
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = temperature;
        self
    }

    #[must_use]
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    fn endpoint(&self) -> String {
        let trimmed = self.api_base.trim_end_matches('/');
        format!("{trimmed}/chat/completions")
    }

    fn build_body(&self, messages: &[ChatMessage]) -> OpenAiChatCompletionRequestDto {
        OpenAiChatCompletionRequestDto {
            model: self.model.clone(),
            messages: messages
                .iter()
                .map(|message| OpenAiChatCompletionRequestMessageDto {
                    role: message.role().to_string(),
                    content: message.content().to_string(),
                })
                .collect(),
            temperature: self.temperature,
            max_tokens: self.max_tokens,
        }
    }
}

impl LlmBaseliner for OpenAiCompatibleLlmBaseliner {
    fn complete(&self, messages: &[ChatMessage]) -> Result<String, LlmBaselinerError> {
        let body = self.build_body(messages);
        let endpoint = self.endpoint();

        let mut request = self.client.post(&endpoint).json(&body);
        if let Some(key) = self.api_key.as_deref() {
            request = request.header(AUTHORIZATION, format!("Bearer {key}"));
        }

        let response = request.send().map_err(|err| LlmBaselinerError::Transport {
            adapter: ADAPTER,
            message: err.to_string(),
        })?;

        let status = response.status();
        let body_text = response
            .text()
            .map_err(|err| LlmBaselinerError::Transport {
                adapter: ADAPTER,
                message: format!("failed to read response body: {err}"),
            })?;

        if !status.is_success() {
            if let Ok(parsed) =
                serde_json::from_str::<OpenAiChatCompletionErrorEnvelopeDto>(&body_text)
            {
                return Err(LlmBaselinerError::ApiError {
                    adapter: ADAPTER,
                    code: parsed.error.code.or(parsed.error.kind),
                    message: parsed.error.message,
                });
            }
            return Err(LlmBaselinerError::ApiError {
                adapter: ADAPTER,
                code: Some(status.as_u16().to_string()),
                message: format!("HTTP {status}: {body_text}"),
            });
        }

        let parsed: OpenAiChatCompletionResponseDto =
            serde_json::from_str(&body_text).map_err(|err| LlmBaselinerError::Protocol {
                adapter: ADAPTER,
                message: format!("response is not a chat-completions envelope: {err}"),
            })?;

        let content = parsed
            .choices
            .into_iter()
            .next()
            .ok_or(LlmBaselinerError::Protocol {
                adapter: ADAPTER,
                message: "response envelope contained no choices".to_string(),
            })?
            .message
            .content;

        Ok(content)
    }
}
