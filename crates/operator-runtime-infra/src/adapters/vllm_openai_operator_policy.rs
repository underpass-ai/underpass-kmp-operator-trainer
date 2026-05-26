use std::fs;
use std::path::Path;

use operator_runtime_application::errors::operator_policy_error::OperatorPolicyError;
use operator_runtime_application::ports::operator_policy_port::OperatorPolicy;
use operator_shared_contract::operator_action_dto::OperatorActionDto;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_infra::mappers::operator_action_mapper::OperatorActionMapper;
use operator_synthetic_domain::calibration::calibration_subject::CalibrationSubject;
use operator_synthetic_infra::adapters::operator_action_schema::vllm_operator_action_schema;
use operator_synthetic_infra::dto::openai_chat_completion_response_dto::OpenAiChatCompletionResponseDto;
use operator_synthetic_infra::mappers::calibration_subject_mapper::CalibrationSubjectMapper;
use reqwest::blocking::Client;
use reqwest::header::CONTENT_TYPE;
use serde::Deserialize;
use serde_json::{Map, Value, json};

use crate::adapters::vllm_operator_config::DEFAULT_MAX_TOKENS;
use crate::adapters::vllm_operator_config::VllmOperatorConfig;
use crate::errors::runtime_infra_error::RuntimeInfraError;

const DEFAULT_SYSTEM_PROMPT: &str = "You are the Operator runtime policy. Return exactly one strict JSON OperatorAction for the provided subject. Use only allowed tools and never emit write tools in read mode.";

#[derive(Debug)]
pub struct VllmOpenAiOperatorPolicy {
    base_url: String,
    model_id: String,
    client: Client,
    max_tokens: u32,
    system_prompt: String,
}

impl VllmOpenAiOperatorPolicy {
    pub fn new(config: &VllmOperatorConfig) -> Result<Self, RuntimeInfraError> {
        let mut builder = Client::builder()
            .timeout(config.timeout())
            .danger_accept_invalid_certs(config.accept_invalid_certs());
        match (config.client_cert_path(), config.client_key_path()) {
            (Some(cert), Some(key)) => {
                builder = builder.identity(load_identity(cert, key)?);
            }
            (None, None) => {}
            (Some(_), None) | (None, Some(_)) => {
                return Err(RuntimeInfraError::IncompleteMtlsConfig);
            }
        }
        let client = builder
            .build()
            .map_err(|err| RuntimeInfraError::HttpClient {
                message: err.to_string(),
            })?;
        Ok(Self::with_client(
            config.base_url(),
            config.model_id(),
            client,
            config.max_tokens(),
        ))
    }

    pub fn with_client(
        base_url: impl Into<String>,
        model_id: impl Into<String>,
        client: Client,
        max_tokens: u32,
    ) -> Self {
        Self {
            base_url: base_url.into(),
            model_id: model_id.into(),
            client,
            max_tokens,
            system_prompt: DEFAULT_SYSTEM_PROMPT.to_string(),
        }
    }

    pub fn for_testing() -> Self {
        Self::with_client(
            "https://0.5b.llm.underpassai.com/v1",
            "operator-v8.1.2",
            Client::builder().build().expect("test client builds"),
            DEFAULT_MAX_TOKENS,
        )
    }

    #[must_use]
    pub fn with_system_prompt(mut self, system_prompt: impl Into<String>) -> Self {
        let system_prompt = system_prompt.into();
        if !system_prompt.trim().is_empty() {
            self.system_prompt = system_prompt;
        }
        self
    }

    pub fn build_request_body(
        &self,
        subject: &CalibrationSubject,
    ) -> Result<Value, OperatorPolicyError> {
        let user_prompt = Self::user_prompt(subject)?;
        Ok(json!({
            "model": self.model_id,
            "messages": [
                {"role": "system", "content": self.system_prompt},
                {"role": "user", "content": user_prompt}
            ],
            "response_format": {
                "type": "json_schema",
                "json_schema": vllm_operator_action_schema()
            },
            "max_tokens": self.max_tokens,
            "temperature": 0.0
        }))
    }

    pub fn parse_response_str(&self, body: &str) -> Result<OperatorAction, OperatorPolicyError> {
        let response: OpenAiChatCompletionResponseDto =
            serde_json::from_str(body).map_err(|err| OperatorPolicyError::Protocol {
                message: format!("chat completion response is not JSON: {err}"),
            })?;
        let choice = response
            .choices
            .first()
            .ok_or_else(|| OperatorPolicyError::Protocol {
                message: "chat completion response has no choices".to_string(),
            })?;
        if let Some(finish_reason) = choice.finish_reason.as_deref()
            && finish_reason != "stop"
        {
            return Err(OperatorPolicyError::Protocol {
                message: format!(
                    "vLLM stopped with finish_reason={finish_reason}; completion may be truncated"
                ),
            });
        }
        parse_action_content(&choice.message.content)
    }

    fn endpoint(&self) -> String {
        let trimmed = self.base_url.trim_end_matches('/');
        format!("{trimmed}/chat/completions")
    }

    fn user_prompt(subject: &CalibrationSubject) -> Result<String, OperatorPolicyError> {
        let subject_dto = CalibrationSubjectMapper::to_dto(subject);
        let value =
            serde_json::to_value(&subject_dto).map_err(|err| OperatorPolicyError::Protocol {
                message: format!("serialize calibration subject: {err}"),
            })?;
        serde_json::to_string(&sort_json_value(value)).map_err(|err| {
            OperatorPolicyError::Protocol {
                message: format!("serialize calibration subject: {err}"),
            }
        })
    }
}

fn sort_json_value(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.into_iter().map(sort_json_value).collect()),
        Value::Object(map) => {
            let mut entries = map.into_iter().collect::<Vec<_>>();
            entries.sort_by(|(left, _), (right, _)| left.cmp(right));
            let mut sorted = Map::new();
            for (key, value) in entries {
                sorted.insert(key, sort_json_value(value));
            }
            Value::Object(sorted)
        }
        other => other,
    }
}

impl OperatorPolicy for VllmOpenAiOperatorPolicy {
    fn predict(&self, subject: &CalibrationSubject) -> Result<OperatorAction, OperatorPolicyError> {
        let body = self.build_request_body(subject)?;
        let response = self
            .client
            .post(self.endpoint())
            .header(CONTENT_TYPE, "application/json")
            .json(&body)
            .send()
            .map_err(|err| OperatorPolicyError::Transport {
                message: err.to_string(),
            })?;
        let status = response.status();
        if !status.is_success() {
            let body = response
                .text()
                .unwrap_or_else(|err| format!("failed to read vLLM error response body: {err}"));
            return Err(OperatorPolicyError::Protocol {
                message: format!("vLLM returned HTTP status {status}: {body}"),
            });
        }
        let text = response
            .text()
            .map_err(|err| OperatorPolicyError::Transport {
                message: err.to_string(),
            })?;
        self.parse_response_str(&text)
    }
}

fn load_identity(
    cert_path: &Path,
    key_path: &Path,
) -> Result<reqwest::Identity, RuntimeInfraError> {
    let mut pem = fs::read(cert_path).map_err(|err| RuntimeInfraError::TlsFileRead {
        path: cert_path.to_path_buf(),
        message: err.to_string(),
    })?;
    let mut key = fs::read(key_path).map_err(|err| RuntimeInfraError::TlsFileRead {
        path: key_path.to_path_buf(),
        message: err.to_string(),
    })?;
    pem.push(b'\n');
    pem.append(&mut key);
    reqwest::Identity::from_pem(&pem).map_err(|err| RuntimeInfraError::TlsIdentity {
        message: err.to_string(),
    })
}

fn parse_action_content(content: &str) -> Result<OperatorAction, OperatorPolicyError> {
    let value: Value =
        serde_json::from_str(content.trim()).map_err(|err| OperatorPolicyError::Shape {
            message: format!("assistant content is not OperatorAction JSON: {err}"),
        })?;
    validate_action_field_set(&value).map_err(|message| OperatorPolicyError::Shape {
        message: format!("{message}; content={}", content.trim()),
    })?;
    let envelope: VllmActionEnvelopeDto =
        serde_json::from_value(value).map_err(|err| OperatorPolicyError::Shape {
            message: format!("assistant content is not OperatorAction JSON: {err}"),
        })?;
    OperatorActionMapper::to_domain(&envelope.action).map_err(|err| OperatorPolicyError::Shape {
        message: format!("assistant action violates domain mapping: {err}"),
    })
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VllmActionEnvelopeDto {
    action: OperatorActionDto,
}

fn validate_action_field_set(value: &Value) -> Result<(), String> {
    let envelope = value
        .as_object()
        .ok_or_else(|| "assistant content must be an object".to_string())?;
    for key in envelope.keys() {
        if key != "action" {
            return Err(format!("assistant envelope field '{key}' is not allowed"));
        }
    }
    let action = value
        .get("action")
        .and_then(Value::as_object)
        .ok_or_else(|| "assistant content must contain object field action".to_string())?;
    let kind = action
        .get("kind")
        .and_then(Value::as_str)
        .ok_or_else(|| "assistant action.kind must be a string".to_string())?;
    let allowed = match kind {
        "tool_call" => &["kind", "tool", "arguments"][..],
        "stop" => &["kind", "reason", "answer", "evidence"][..],
        "escalate" => &["kind", "reason", "target_model"][..],
        other => return Err(format!("assistant action.kind is unsupported: {other}")),
    };
    for required in allowed {
        if !action.contains_key(*required) {
            return Err(format!(
                "assistant action field '{required}' is required for {kind}"
            ));
        }
    }
    for key in action.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(format!(
                "assistant action field '{key}' is not allowed for {kind}"
            ));
        }
    }
    if kind == "tool_call" && !action.get("arguments").is_some_and(Value::is_object) {
        return Err("assistant action.arguments must be an object for tool_call".to_string());
    }
    Ok(())
}
