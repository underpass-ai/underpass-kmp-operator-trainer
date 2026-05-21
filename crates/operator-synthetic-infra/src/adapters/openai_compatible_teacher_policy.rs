//! `TeacherPolicy` adapter for OpenAI-compatible chat-completions APIs.

use std::fs;
use std::path::Path;
use std::time::Duration;

use operator_shared_contract::operator_action_dto::OperatorActionDto;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_infra::mappers::operator_action_mapper::OperatorActionMapper;
use operator_synthetic_application::error::teacher_policy_error::TeacherPolicyError;
use operator_synthetic_application::ports::teacher_policy::TeacherPolicy;
use operator_synthetic_domain::calibration::calibration_subject::CalibrationSubject;
use reqwest::blocking::Client;
use reqwest::header::AUTHORIZATION;

use crate::dto::openai_chat_completion_error_envelope_dto::OpenAiChatCompletionErrorEnvelopeDto;
use crate::dto::openai_chat_completion_request_dto::OpenAiChatCompletionRequestDto;
use crate::dto::openai_chat_completion_request_message_dto::OpenAiChatCompletionRequestMessageDto;
use crate::dto::openai_chat_completion_response_dto::OpenAiChatCompletionResponseDto;
use crate::mappers::calibration_subject_mapper::CalibrationSubjectMapper;

const ADAPTER: &str = "openai_compatible_teacher_policy";
const DEFAULT_TIMEOUT_SECS: u64 = 60;
const DEFAULT_TEMPERATURE: f32 = 0.0;
const DEFAULT_MAX_TOKENS: u32 = 700;

#[derive(Debug)]
pub struct OpenAiCompatibleTeacherPolicy {
    api_base: String,
    api_key: Option<String>,
    model: String,
    temperature: f32,
    max_tokens: u32,
    prompt: String,
    client: Client,
}

impl OpenAiCompatibleTeacherPolicy {
    pub fn new(
        api_base: impl Into<String>,
        api_key: Option<String>,
        model: impl Into<String>,
        prompt_path: &Path,
    ) -> Result<Self, TeacherPolicyError> {
        let prompt =
            fs::read_to_string(prompt_path).map_err(|err| TeacherPolicyError::Protocol {
                adapter: ADAPTER,
                message: format!("read prompt {}: {err}", prompt_path.display()),
            })?;
        let client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .map_err(|err| TeacherPolicyError::Transport {
                adapter: ADAPTER,
                message: format!("failed to build reqwest client: {err}"),
            })?;
        Ok(Self::with_client(api_base, api_key, model, prompt, client))
    }

    pub fn with_client(
        api_base: impl Into<String>,
        api_key: Option<String>,
        model: impl Into<String>,
        prompt: impl Into<String>,
        client: Client,
    ) -> Self {
        Self {
            api_base: api_base.into(),
            api_key,
            model: model.into(),
            temperature: DEFAULT_TEMPERATURE,
            max_tokens: DEFAULT_MAX_TOKENS,
            prompt: prompt.into(),
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
        self.max_tokens = max_tokens;
        self
    }

    fn endpoint(&self) -> String {
        let trimmed = self.api_base.trim_end_matches('/');
        format!("{trimmed}/chat/completions")
    }

    fn build_body(
        &self,
        subject: &CalibrationSubject,
    ) -> Result<OpenAiChatCompletionRequestDto, TeacherPolicyError> {
        let subject_dto = CalibrationSubjectMapper::to_dto(subject);
        let subject_json = serde_json::to_string_pretty(&subject_dto).map_err(|err| {
            TeacherPolicyError::Protocol {
                adapter: ADAPTER,
                message: format!("serialize subject: {err}"),
            }
        })?;
        Ok(OpenAiChatCompletionRequestDto {
            model: self.model.clone(),
            messages: vec![
                OpenAiChatCompletionRequestMessageDto {
                    role: "system".to_string(),
                    content: self.prompt.clone(),
                },
                OpenAiChatCompletionRequestMessageDto {
                    role: "user".to_string(),
                    content: subject_json,
                },
            ],
            temperature: self.temperature,
            max_tokens: Some(self.max_tokens),
        })
    }
}

impl TeacherPolicy for OpenAiCompatibleTeacherPolicy {
    fn decide(&self, subject: &CalibrationSubject) -> Result<OperatorAction, TeacherPolicyError> {
        let body = self.build_body(subject)?;
        let mut request = self.client.post(self.endpoint()).json(&body);
        if let Some(key) = self.api_key.as_deref() {
            request = request.header(AUTHORIZATION, format!("Bearer {key}"));
        }
        let response = request
            .send()
            .map_err(|err| TeacherPolicyError::Transport {
                adapter: ADAPTER,
                message: err.to_string(),
            })?;
        let status = response.status();
        let body_text = response
            .text()
            .map_err(|err| TeacherPolicyError::Transport {
                adapter: ADAPTER,
                message: format!("failed to read response body: {err}"),
            })?;
        if !status.is_success() {
            if let Ok(parsed) =
                serde_json::from_str::<OpenAiChatCompletionErrorEnvelopeDto>(&body_text)
            {
                return Err(TeacherPolicyError::ApiError {
                    adapter: ADAPTER,
                    code: parsed.error.code.or(parsed.error.kind),
                    message: parsed.error.message,
                });
            }
            return Err(TeacherPolicyError::ApiError {
                adapter: ADAPTER,
                code: Some(status.as_u16().to_string()),
                message: format!("HTTP {status}: {body_text}"),
            });
        }
        let parsed: OpenAiChatCompletionResponseDto =
            serde_json::from_str(&body_text).map_err(|err| TeacherPolicyError::Protocol {
                adapter: ADAPTER,
                message: format!("response is not a chat-completions envelope: {err}"),
            })?;
        let content = parsed
            .choices
            .into_iter()
            .next()
            .ok_or(TeacherPolicyError::Protocol {
                adapter: ADAPTER,
                message: "response envelope contained no choices".to_string(),
            })?
            .message
            .content;
        let action_dto: OperatorActionDto =
            serde_json::from_str(content.trim()).map_err(|err| TeacherPolicyError::Shape {
                adapter: ADAPTER,
                message: format!("assistant content is not OperatorActionDto JSON: {err}"),
            })?;
        OperatorActionMapper::to_domain(&action_dto).map_err(|err| TeacherPolicyError::Shape {
            adapter: ADAPTER,
            message: format!("assistant action violates DTO/domain mapping: {err}"),
        })
    }
}
