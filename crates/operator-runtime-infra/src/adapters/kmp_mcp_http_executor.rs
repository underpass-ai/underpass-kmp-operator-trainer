use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use operator_replay_application::error::kmp_client_error::KmpClientError;
use operator_replay_infra::jsonrpc::envelope_violation::EnvelopeViolation;
use operator_replay_infra::jsonrpc::tools_call_request::ToolsCallRequest;
use operator_replay_infra::jsonrpc::tools_call_response::ToolsCallResponse;
use operator_replay_infra::mappers::ask_response_mapper::AskResponseMapper;
use operator_replay_infra::mappers::forward_response_mapper::ForwardResponseMapper;
use operator_replay_infra::mappers::goto_response_mapper::GotoResponseMapper;
use operator_replay_infra::mappers::inspect_response_mapper::InspectResponseMapper;
use operator_replay_infra::mappers::mapping_error::MappingError;
use operator_replay_infra::mappers::near_response_mapper::NearResponseMapper;
use operator_replay_infra::mappers::rewind_response_mapper::RewindResponseMapper;
use operator_replay_infra::mappers::trace_response_mapper::TraceResponseMapper;
use operator_replay_infra::mappers::wake_response_mapper::WakeResponseMapper;
use operator_runtime_application::errors::mcp_executor_error::McpExecutorError;
use operator_runtime_application::ports::mcp_executor_port::McpExecutor;
use operator_runtime_domain::session::observation::Observation;
use operator_runtime_domain::session::observation_error_code::ObservationErrorCode;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::tool::kernel_tool::KernelTool;
use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
use operator_shared_domain::tool_outcomes::tool_outcome::ToolOutcome;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use reqwest::blocking::Client;
use serde_json::Value;

use crate::adapters::kmp_mcp_request_arguments::{self, McpRequestArgumentsError};

const TOOL_ERROR_CODE: &str = "mcp_tool_error";
const DEFAULT_TIMEOUT_SECS: u64 = 30;

#[derive(Debug)]
pub struct KmpMcpHttpExecutor {
    endpoint: String,
    client: Client,
    next_id: AtomicU64,
}

impl KmpMcpHttpExecutor {
    pub fn new(endpoint: impl Into<String>) -> Result<Self, McpExecutorError> {
        let client = Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .map_err(|err| McpExecutorError::Transport {
                message: format!("failed to build reqwest client: {err}"),
            })?;
        Ok(Self::with_client(endpoint, client))
    }

    pub fn with_client(endpoint: impl Into<String>, client: Client) -> Self {
        Self {
            endpoint: endpoint.into(),
            client,
            next_id: AtomicU64::new(1),
        }
    }

    fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    fn call_tool(
        &self,
        tool: KernelTool,
        arguments: Value,
    ) -> Result<Observation, McpExecutorError> {
        let request_id = self.next_id();
        let request = ToolsCallRequest::new(request_id, tool.as_str(), arguments);
        let response = self
            .client
            .post(&self.endpoint)
            .json(&request)
            .send()
            .map_err(|err| McpExecutorError::Transport {
                message: err.to_string(),
            })?;
        let status = response.status();
        let body = response.text().map_err(|err| McpExecutorError::Transport {
            message: format!("failed to read MCP HTTP response body: {err}"),
        })?;
        if !status.is_success() {
            return Err(McpExecutorError::Protocol {
                message: format!("MCP HTTP status {status}: {body}"),
            });
        }
        let envelope: ToolsCallResponse =
            serde_json::from_str(&body).map_err(|err| McpExecutorError::Protocol {
                message: format!("invalid JSON-RPC envelope from HTTP MCP: {err}"),
            })?;
        if let Err(violation) = envelope.validate(request_id) {
            return Err(McpExecutorError::Protocol {
                message: render_envelope_violation(&violation),
            });
        }
        if let Some(error) = envelope.error.as_ref() {
            return Err(McpExecutorError::Protocol {
                message: format!("JSON-RPC error {}: {}", error.code, error.message),
            });
        }
        if let Some(result) = envelope.result.as_ref().filter(|result| result.is_error) {
            return Ok(Observation::ToolError {
                code: ObservationErrorCode::parse(TOOL_ERROR_CODE)
                    .expect("static observation error code is valid"),
                message: tool_error_message(result),
            });
        }

        let structured = envelope
            .structured_content()
            .map_err(|message| McpExecutorError::Protocol { message })?;
        let outcome = map_structured_content(tool, &structured).map_err(map_kmp_error)?;
        Ok(tool_response(outcome, &structured))
    }

    fn call_read_tool(
        &self,
        tool: KernelTool,
        arguments: Result<Value, McpRequestArgumentsError>,
    ) -> Result<Observation, McpExecutorError> {
        match arguments {
            Ok(arguments) => self.call_tool(tool, arguments),
            Err(error) => Ok(mcp_argument_error_observation(&error)),
        }
    }
}

impl McpExecutor for KmpMcpHttpExecutor {
    fn execute(
        &self,
        action: &OperatorAction,
        about: &AboutId,
    ) -> Result<Observation, McpExecutorError> {
        let OperatorAction::ToolCall(call) = action else {
            return Err(McpExecutorError::UnsupportedTool {
                tool: "terminal_action",
            });
        };

        match call.arguments() {
            ToolArguments::Ingest(_) | ToolArguments::WriteMemory(_) => {
                Err(McpExecutorError::WriteToolNotAllowedInReadProfile)
            }
            ToolArguments::Wake(args) => {
                self.call_tool(KernelTool::Wake, kmp_mcp_request_arguments::wake(args))
            }
            ToolArguments::Ask(args) => {
                self.call_tool(KernelTool::Ask, kmp_mcp_request_arguments::ask(args, about))
            }
            ToolArguments::Near(args) => self.call_tool(
                KernelTool::Near,
                kmp_mcp_request_arguments::near(args, about),
            ),
            ToolArguments::Goto(args) => self.call_read_tool(
                KernelTool::Goto,
                kmp_mcp_request_arguments::goto(args, about),
            ),
            ToolArguments::Rewind(args) => self.call_read_tool(
                KernelTool::Rewind,
                kmp_mcp_request_arguments::rewind(args, about),
            ),
            ToolArguments::Forward(args) => self.call_read_tool(
                KernelTool::Forward,
                kmp_mcp_request_arguments::forward(args, about),
            ),
            ToolArguments::Trace(args) => {
                self.call_read_tool(KernelTool::Trace, kmp_mcp_request_arguments::trace(args))
            }
            ToolArguments::Inspect(args) => self.call_tool(
                KernelTool::Inspect,
                kmp_mcp_request_arguments::inspect(args),
            ),
        }
    }
}

fn map_structured_content(
    tool: KernelTool,
    structured: &Value,
) -> Result<ToolOutcome, KmpClientError> {
    match tool {
        KernelTool::Ingest | KernelTool::WriteMemory => Err(KmpClientError::Protocol {
            tool: tool.as_str(),
            message: "write tool reached read-profile HTTP executor".to_string(),
        }),
        KernelTool::Wake => WakeResponseMapper::to_outcome(structured)
            .map(ToolOutcome::Wake)
            .map_err(|e| into_kmp_client_error(tool, &e)),
        KernelTool::Ask => AskResponseMapper::to_outcome(structured)
            .map(ToolOutcome::Ask)
            .map_err(|e| into_kmp_client_error(tool, &e)),
        KernelTool::Near => NearResponseMapper::to_outcome(structured)
            .map(ToolOutcome::Near)
            .map_err(|e| into_kmp_client_error(tool, &e)),
        KernelTool::Goto => GotoResponseMapper::to_outcome(structured)
            .map(ToolOutcome::Goto)
            .map_err(|e| into_kmp_client_error(tool, &e)),
        KernelTool::Rewind => RewindResponseMapper::to_outcome(structured)
            .map(ToolOutcome::Rewind)
            .map_err(|e| into_kmp_client_error(tool, &e)),
        KernelTool::Forward => ForwardResponseMapper::to_outcome(structured)
            .map(ToolOutcome::Forward)
            .map_err(|e| into_kmp_client_error(tool, &e)),
        KernelTool::Trace => TraceResponseMapper::to_outcome(structured)
            .map(ToolOutcome::Trace)
            .map_err(|e| into_kmp_client_error(tool, &e)),
        KernelTool::Inspect => InspectResponseMapper::to_outcome(structured)
            .map(ToolOutcome::Inspect)
            .map_err(|e| into_kmp_client_error(tool, &e)),
    }
}

fn map_kmp_error(error: KmpClientError) -> McpExecutorError {
    match error {
        KmpClientError::Transport { message, .. } => McpExecutorError::Transport { message },
        KmpClientError::Protocol { message, .. }
        | KmpClientError::InvalidArguments { message, .. }
        | KmpClientError::MalformedResponse { message, .. } => {
            McpExecutorError::Protocol { message }
        }
    }
}

fn into_kmp_client_error(tool: KernelTool, err: &MappingError) -> KmpClientError {
    KmpClientError::MalformedResponse {
        tool: tool.as_str(),
        message: err.to_string(),
    }
}

fn mcp_argument_error_observation(error: &McpRequestArgumentsError) -> Observation {
    Observation::ToolError {
        code: ObservationErrorCode::parse(TOOL_ERROR_CODE)
            .expect("static observation error code is valid"),
        message: format!("invalid MCP request arguments: {error}"),
    }
}

fn render_envelope_violation(violation: &EnvelopeViolation) -> String {
    match violation {
        EnvelopeViolation::WrongJsonRpcVersion { actual } => {
            format!("jsonrpc field is '{actual}', expected '2.0'")
        }
        EnvelopeViolation::MissingId => "JSON-RPC response missing id".to_string(),
        EnvelopeViolation::IdMismatch { expected, actual } => {
            format!("JSON-RPC id mismatch: request id was {expected}, response id is {actual}")
        }
        EnvelopeViolation::ResultAndErrorBothPresent => {
            "JSON-RPC response carries both result and error".to_string()
        }
        EnvelopeViolation::NeitherResultNorError => {
            "JSON-RPC response carries neither result nor error".to_string()
        }
    }
}

fn tool_error_message(
    result: &operator_replay_infra::jsonrpc::tools_call_result::ToolsCallResult,
) -> String {
    result
        .content
        .first()
        .map(|content| content.text.clone())
        .filter(|message| !message.trim().is_empty())
        .unwrap_or_else(|| "MCP tool returned isError=true".to_string())
}

fn tool_response(outcome: ToolOutcome, structured: &Value) -> Observation {
    let observed_refs = observed_refs_for(&outcome);
    let signals = crate::adapters::mcp_navigation_signals::navigation_signals_from_structured(
        structured,
        &observed_refs,
    );
    Observation::ToolResponse {
        outcome,
        observed_refs,
        signals,
    }
}

fn observed_refs_for(outcome: &ToolOutcome) -> Vec<MemoryRef> {
    match outcome {
        ToolOutcome::Ingest(_) | ToolOutcome::WriteMemory(_) => vec![],
        ToolOutcome::Wake(o) => o.surfaced_refs().to_vec(),
        ToolOutcome::Ask(o) => o.evidence_refs().to_vec(),
        ToolOutcome::Near(o) => o.entry_refs().to_vec(),
        ToolOutcome::Goto(o) => o.entry_refs().to_vec(),
        ToolOutcome::Rewind(o) => o.entry_refs().to_vec(),
        ToolOutcome::Forward(o) => o.entry_refs().to_vec(),
        ToolOutcome::Trace(o) => o.path_refs().to_vec(),
        ToolOutcome::Inspect(o) => {
            let mut refs =
                Vec::with_capacity(1 + o.incoming_refs().len() + o.outgoing_refs().len());
            refs.push(o.target().clone());
            refs.extend(o.incoming_refs().iter().cloned());
            refs.extend(o.outgoing_refs().iter().cloned());
            refs
        }
    }
}
