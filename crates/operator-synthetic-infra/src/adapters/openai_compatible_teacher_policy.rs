//! `TeacherPolicy` adapter for OpenAI-compatible chat-completions APIs.

use std::fs;
use std::path::Path;
use std::time::Duration;

use operator_shared_contract::operator_action_dto::OperatorActionDto;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::value_objects::finish_reason::FinishReason;
use operator_shared_domain::value_objects::subject_hash::SubjectHash;
use operator_shared_infra::mappers::operator_action_mapper::OperatorActionMapper;
use operator_synthetic_application::error::teacher_policy_error::TeacherPolicyError;
use operator_synthetic_application::ports::teacher_policy::TeacherPolicy;
use operator_synthetic_domain::calibration::calibration_subject::CalibrationSubject;
use operator_synthetic_domain::calibration::teacher_decision::TeacherDecision;
use reqwest::blocking::Client;
use reqwest::header::AUTHORIZATION;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::adapters::operator_action_schema::operator_action_schema;
use crate::dto::openai_chat_completion_error_envelope_dto::OpenAiChatCompletionErrorEnvelopeDto;
use crate::dto::openai_chat_completion_request_dto::OpenAiChatCompletionRequestDto;
use crate::dto::openai_chat_completion_request_message_dto::OpenAiChatCompletionRequestMessageDto;
use crate::dto::openai_chat_completion_response_dto::OpenAiChatCompletionResponseDto;
use crate::mappers::calibration_subject_mapper::CalibrationSubjectMapper;

const ADAPTER: &str = "openai_compatible_teacher_policy";
const DEFAULT_TIMEOUT_SECS: u64 = 180;
const DEFAULT_TEMPERATURE: f32 = 0.0;
// The 2026-05-23 full v7.3 run saw two structured-output truncations at
// content_len=4205 with the old 4096 ceiling. Accepted action p99 was ~2 KB,
// so 8192 leaves headroom while future length finishes remain observable.
const DEFAULT_MAX_TOKENS: u32 = 8192;

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

    fn build_body(&self, subject_json: String) -> OpenAiChatCompletionRequestDto {
        OpenAiChatCompletionRequestDto {
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
            max_tokens: None,
            max_completion_tokens: Some(self.max_tokens),
            response_format: Some(serde_json::json!({
                "type": "json_schema",
                "json_schema": operator_action_schema(),
            })),
        }
    }

    fn subject_json(subject: &CalibrationSubject) -> Result<String, TeacherPolicyError> {
        let subject_dto = CalibrationSubjectMapper::to_dto(subject);
        serde_json::to_string_pretty(&subject_dto).map_err(|err| TeacherPolicyError::Protocol {
            adapter: ADAPTER,
            message: format!("serialize subject: {err}"),
        })
    }

    fn subject_hash(subject_json: &str) -> Result<SubjectHash, TeacherPolicyError> {
        let mut hasher = Sha256::new();
        hasher.update(subject_json.as_bytes());
        SubjectHash::parse(format!("{:x}", hasher.finalize())).map_err(|err| {
            TeacherPolicyError::Protocol {
                adapter: ADAPTER,
                message: format!("build subject hash: {err}"),
            }
        })
    }
}

impl TeacherPolicy for OpenAiCompatibleTeacherPolicy {
    fn decide(&self, subject: &CalibrationSubject) -> Result<TeacherDecision, TeacherPolicyError> {
        let subject_json = Self::subject_json(subject)?;
        let subject_hash = Self::subject_hash(&subject_json)?;
        if let Some(prepared) = subject.prepared_action() {
            return Ok(TeacherDecision::new(
                prepared.action().clone(),
                FinishReason::Stop,
                subject_hash,
            ));
        }

        let body = self.build_body(subject_json);
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
                let message = parsed.error.message;
                if is_structured_output_error(&message) {
                    return Err(TeacherPolicyError::ApiError {
                        adapter: ADAPTER,
                        code: Some("structured_output_not_supported".to_string()),
                        message: format!(
                            "OpenAI rejected json_schema response_format; check model + API version: {message}"
                        ),
                    });
                }
                return Err(TeacherPolicyError::ApiError {
                    adapter: ADAPTER,
                    code: parsed.error.code.or(parsed.error.kind),
                    message,
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
        let choice = parsed
            .choices
            .into_iter()
            .next()
            .ok_or(TeacherPolicyError::Protocol {
                adapter: ADAPTER,
                message: "response envelope contained no choices".to_string(),
            })?;
        let finish_reason = choice
            .finish_reason
            .as_deref()
            .map_or(FinishReason::Other, FinishReason::parse);
        let content = choice.message.content;
        if finish_reason != FinishReason::Stop {
            return Err(TeacherPolicyError::TruncatedResponse {
                adapter: ADAPTER,
                finish_reason,
                content_len: content.chars().count(),
            });
        }
        let action = parse_action_envelope(&content, finish_reason)?;
        Ok(TeacherDecision::new(action, finish_reason, subject_hash))
    }
}

fn parse_action_envelope(
    content: &str,
    finish_reason: FinishReason,
) -> Result<OperatorAction, TeacherPolicyError> {
    let action_value: Value =
        serde_json::from_str(content.trim()).map_err(|err| TeacherPolicyError::Shape {
            adapter: ADAPTER,
            message: format!(
                "assistant content is not OperatorAction envelope JSON: {err}; finish_reason={}; content_len={}; content_tail={}",
                finish_reason.as_str(),
                content.chars().count(),
                content_tail(content),
            ),
            finish_reason: Some(finish_reason),
        })?;
    let action_value =
        action_value
            .get("action")
            .cloned()
            .ok_or_else(|| TeacherPolicyError::Shape {
                adapter: ADAPTER,
                message: "assistant content missing required action envelope".to_string(),
                finish_reason: Some(finish_reason),
            })?;
    let action_dto: OperatorActionDto =
        serde_json::from_value(action_value).map_err(|err| TeacherPolicyError::Shape {
            adapter: ADAPTER,
            message: format!("assistant action is not OperatorActionDto JSON: {err}"),
            finish_reason: Some(finish_reason),
        })?;
    OperatorActionMapper::to_domain(&action_dto).map_err(|err| TeacherPolicyError::Shape {
        adapter: ADAPTER,
        message: format!("assistant action violates DTO/domain mapping: {err}"),
        finish_reason: Some(finish_reason),
    })
}

fn is_structured_output_error(message: &str) -> bool {
    let lower = message.to_lowercase();
    lower.contains("response_format") || lower.contains("json_schema")
}

fn content_tail(content: &str) -> String {
    let mut chars: Vec<char> = content.chars().rev().take(256).collect();
    chars.reverse();
    chars.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;
    use std::sync::mpsc;
    use std::thread::{self, JoinHandle};
    use std::time::Duration;

    use operator_shared_domain::action::operator_action::OperatorAction;
    use operator_shared_domain::action::tool_call_action::ToolCallAction;
    use operator_shared_domain::ids::about_id::AboutId;
    use operator_shared_domain::mode::allowed_tools::AllowedTools;
    use operator_shared_domain::mode::operator_mode::OperatorMode;
    use operator_shared_domain::tool::kernel_tool::KernelTool;
    use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
    use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
    use operator_shared_domain::value_objects::memory_ref::MemoryRef;
    use operator_shared_domain::value_objects::task_family::TaskFamily;
    use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
    use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
    use operator_shared_domain::visible_state::visible_state::VisibleState;
    use operator_synthetic_application::ports::teacher_policy::TeacherPolicy;
    use operator_synthetic_domain::calibration::prepared_operator_action::PreparedOperatorAction;
    use serde_json::Value;

    use super::*;

    fn spawn_mock_server_with_status(
        status_line: &'static str,
        response_body: String,
    ) -> (String, JoinHandle<()>, mpsc::Receiver<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let port = listener.local_addr().expect("local addr").port();
        let url = format!("http://127.0.0.1:{port}");
        let (tx, rx) = mpsc::channel();
        let handle = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept connection");
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("set read timeout");
            let mut reader = BufReader::new(&mut stream);
            let mut content_length: usize = 0;
            loop {
                let mut line = String::new();
                let bytes = reader.read_line(&mut line).expect("read header");
                if bytes == 0 || line == "\r\n" || line == "\n" {
                    break;
                }
                let lower = line.to_ascii_lowercase();
                if let Some(rest) = lower.strip_prefix("content-length:") {
                    content_length = rest.trim().parse().unwrap_or(0);
                }
            }
            let mut body_bytes = vec![0u8; content_length];
            reader.read_exact(&mut body_bytes).expect("read body");
            let request_body = String::from_utf8(body_bytes).unwrap_or_default();
            let _ = tx.send(request_body);
            drop(reader);
            let response = format!(
                "HTTP/1.1 {status_line}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response_body.len(),
                response_body,
            );
            stream
                .write_all(response.as_bytes())
                .expect("write response");
            stream.flush().expect("flush response");
        });
        (url, handle, rx)
    }

    fn subject() -> CalibrationSubject {
        CalibrationSubject::new(
            AboutId::parse("about:test").expect("about parses"),
            OperatorMode::Read,
            TaskFamily::parse("read.inspect").expect("task family parses"),
            TrajectoryGoal::parse("Inspect the visible node.").expect("goal parses"),
            AllowedTools::for_mode(OperatorMode::Read),
            VisibleState::assemble([], [], None, BudgetSnapshot::unbounded()),
            None,
        )
        .expect("subject builds")
    }

    fn prepared_subject(action: OperatorAction) -> CalibrationSubject {
        CalibrationSubject::new(
            AboutId::parse("about:test").expect("about parses"),
            OperatorMode::Read,
            TaskFamily::parse("read.inspect").expect("task family parses"),
            TrajectoryGoal::parse("Execute the prepared inspect action.").expect("goal parses"),
            AllowedTools::for_mode(OperatorMode::Read),
            VisibleState::assemble(
                [MemoryRef::parse("about:test:node:x").expect("ref parses")],
                [],
                None,
                BudgetSnapshot::unbounded(),
            ),
            Some(PreparedOperatorAction::new(action).expect("prepared action builds")),
        )
        .expect("subject builds")
    }

    #[test]
    fn request_body_includes_structured_response_format() {
        let action = serde_json::json!({
            "kind": "tool_call",
            "tool": "kernel_inspect",
            "arguments": {
                "target": "about:test:node:x"
            },
            "reason": "none",
            "answer": null,
            "evidence": [],
            "target_model": "none"
        });
        let response = serde_json::json!({
            "choices": [
                {
                    "message": {
                        "content": serde_json::json!({ "action": action }).to_string()
                    },
                    "finish_reason": "stop"
                }
            ]
        })
        .to_string();
        let (url, handle, rx) = spawn_mock_server_with_status("200 OK", response);
        let policy = OpenAiCompatibleTeacherPolicy::with_client(
            url,
            None,
            "gpt-4o-mini",
            "Return one operator action.",
            Client::new(),
        );

        let decision = policy.decide(&subject()).expect("teacher decision maps");
        let request: Value =
            serde_json::from_str(&rx.recv().expect("request body")).expect("request body is json");
        handle.join().expect("mock server joins");

        assert!(matches!(decision.action(), OperatorAction::ToolCall(_)));
        assert_eq!(
            decision.action().tool(),
            Some(KernelTool::Inspect),
            "mock response should map through the normal DTO/domain path"
        );
        assert_eq!(decision.finish_reason(), FinishReason::Stop);
        assert_eq!(
            request["response_format"]["type"],
            Value::String("json_schema".to_string())
        );
        assert_eq!(
            request["response_format"]["json_schema"]["name"],
            Value::String("OperatorAction".to_string())
        );
        assert_eq!(
            request["response_format"]["json_schema"]["strict"],
            Value::Bool(true)
        );
        assert_eq!(request["max_completion_tokens"], serde_json::json!(8192));
        assert!(request.get("max_tokens").is_none());
    }

    #[test]
    fn length_finish_reason_returns_truncated_response_without_parsing() {
        let response = serde_json::json!({
            "choices": [
                {
                    "message": {
                        "content": "{"
                    },
                    "finish_reason": "length"
                }
            ]
        })
        .to_string();
        let (url, handle, _rx) = spawn_mock_server_with_status("200 OK", response);
        let policy = OpenAiCompatibleTeacherPolicy::with_client(
            url,
            None,
            "gpt-4o-mini",
            "Return one operator action.",
            Client::new(),
        );

        let err = policy
            .decide(&subject())
            .expect_err("length finish should be explicit truncation");
        handle.join().expect("mock server joins");

        assert_eq!(
            err,
            TeacherPolicyError::TruncatedResponse {
                adapter: ADAPTER,
                finish_reason: FinishReason::Length,
                content_len: 1,
            }
        );
    }

    #[test]
    fn content_filter_finish_reason_returns_truncated_response_without_parsing() {
        let response = serde_json::json!({
            "choices": [
                {
                    "message": {
                        "content": "{}"
                    },
                    "finish_reason": "content_filter"
                }
            ]
        })
        .to_string();
        let (url, handle, _rx) = spawn_mock_server_with_status("200 OK", response);
        let policy = OpenAiCompatibleTeacherPolicy::with_client(
            url,
            None,
            "gpt-4o-mini",
            "Return one operator action.",
            Client::new(),
        );

        let err = policy
            .decide(&subject())
            .expect_err("content filter finish should be explicit truncation");
        handle.join().expect("mock server joins");

        assert_eq!(
            err,
            TeacherPolicyError::TruncatedResponse {
                adapter: ADAPTER,
                finish_reason: FinishReason::ContentFilter,
                content_len: 2,
            }
        );
    }

    #[test]
    fn prepared_action_returns_without_http_call() {
        let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
            InspectArguments::new(MemoryRef::parse("about:test:node:x").expect("ref parses")),
        )));
        let policy = OpenAiCompatibleTeacherPolicy::with_client(
            "http://127.0.0.1:9",
            None,
            "gpt-4o-mini",
            "Return one operator action.",
            Client::builder()
                .timeout(Duration::from_millis(1))
                .build()
                .expect("client builds"),
        );

        let decision = policy
            .decide(&prepared_subject(action.clone()))
            .expect("prepared action returns directly");

        assert_eq!(decision.action(), &action);
        assert_eq!(decision.finish_reason(), FinishReason::Stop);
    }

    #[test]
    fn response_format_rejection_maps_to_structured_output_not_supported() {
        let response = serde_json::json!({
            "error": {
                "message": "Invalid schema for response_format 'OperatorAction': json_schema is unsupported",
                "type": "invalid_request_error",
                "code": null
            }
        })
        .to_string();
        let (url, handle, _rx) = spawn_mock_server_with_status("400 Bad Request", response);
        let policy = OpenAiCompatibleTeacherPolicy::with_client(
            url,
            None,
            "legacy-model",
            "Return one operator action.",
            Client::new(),
        );

        let error = policy
            .decide(&subject())
            .expect_err("response_format rejection should be explicit");
        handle.join().expect("mock server joins");

        assert_eq!(
            error,
            TeacherPolicyError::ApiError {
                adapter: ADAPTER,
                code: Some("structured_output_not_supported".to_string()),
                message: "OpenAI rejected json_schema response_format; check model + API version: Invalid schema for response_format 'OperatorAction': json_schema is unsupported".to_string(),
            }
        );
    }
}
