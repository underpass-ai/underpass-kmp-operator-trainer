use std::io::Write;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};

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
use operator_shared_domain::cursor::cursor::Cursor;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::tool::kernel_tool::KernelTool;
use operator_shared_domain::tool_arguments::ask_arguments::AskArguments;
use operator_shared_domain::tool_arguments::forward_arguments::ForwardArguments;
use operator_shared_domain::tool_arguments::goto_arguments::GotoArguments;
use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
use operator_shared_domain::tool_arguments::near_arguments::NearArguments;
use operator_shared_domain::tool_arguments::rewind_arguments::RewindArguments;
use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
use operator_shared_domain::tool_arguments::trace_arguments::TraceArguments;
use operator_shared_domain::tool_arguments::wake_arguments::WakeArguments;
use operator_shared_domain::tool_outcomes::tool_outcome::ToolOutcome;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use serde_json::{Value, json};

use crate::adapters::kmp_mcp_stdio_config::KmpMcpStdioConfig;

const TOOL_ERROR_CODE: &str = "mcp_tool_error";

#[derive(Debug)]
pub struct KmpMcpStdioExecutor {
    config: KmpMcpStdioConfig,
    next_id: AtomicU64,
}

impl KmpMcpStdioExecutor {
    pub fn new(config: KmpMcpStdioConfig) -> Self {
        Self {
            config,
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
        let request_line =
            serde_json::to_string(&request).map_err(|err| McpExecutorError::Protocol {
                message: format!("failed to serialize JSON-RPC request: {err}"),
            })?;
        let response_line = self.invoke_stdio(&request_line)?;
        let envelope: ToolsCallResponse =
            serde_json::from_str(&response_line).map_err(|err| McpExecutorError::Protocol {
                message: format!("invalid JSON-RPC envelope from stdio MCP: {err}"),
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
        Ok(tool_response(outcome))
    }

    fn invoke_stdio(&self, request_line: &str) -> Result<String, McpExecutorError> {
        let mut child = Command::new(self.config.command())
            .args(self.config.args())
            .env("REHYDRATION_MCP_BACKEND", "grpc")
            .env(
                "REHYDRATION_KERNEL_GRPC_ENDPOINT",
                self.config.grpc_endpoint(),
            )
            .envs(self.config.env())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|err| McpExecutorError::Transport {
                message: format!("failed to spawn {}: {err}", self.config.command().display()),
            })?;

        {
            let stdin = child
                .stdin
                .as_mut()
                .ok_or_else(|| McpExecutorError::Transport {
                    message: "failed to open stdin for stdio MCP process".to_string(),
                })?;
            writeln!(stdin, "{request_line}").map_err(|err| McpExecutorError::Transport {
                message: format!("failed to write JSON-RPC request to stdio MCP process: {err}"),
            })?;
        }

        let output = child
            .wait_with_output()
            .map_err(|err| McpExecutorError::Transport {
                message: format!("failed to wait for stdio MCP process: {err}"),
            })?;
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !output.status.success() {
            return Err(McpExecutorError::Transport {
                message: format!(
                    "stdio MCP process exited with {}; stderr: {}",
                    output.status,
                    stderr.trim()
                ),
            });
        }
        let stdout = String::from_utf8_lossy(&output.stdout);
        stdout
            .lines()
            .find(|line| !line.trim().is_empty())
            .map(str::to_string)
            .ok_or_else(|| McpExecutorError::Protocol {
                message: format!(
                    "stdio MCP process produced no JSON-RPC response; stderr: {}",
                    stderr.trim()
                ),
            })
    }
}

impl McpExecutor for KmpMcpStdioExecutor {
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
            ToolArguments::Wake(args) => self.call_tool(KernelTool::Wake, wake_arguments(args)),
            ToolArguments::Ask(args) => self.call_tool(KernelTool::Ask, ask_arguments(args, about)),
            ToolArguments::Near(args) => {
                self.call_tool(KernelTool::Near, near_arguments(args, about))
            }
            ToolArguments::Goto(args) => {
                self.call_tool(KernelTool::Goto, goto_arguments(args, about))
            }
            ToolArguments::Rewind(args) => {
                self.call_tool(KernelTool::Rewind, rewind_arguments(args, about))
            }
            ToolArguments::Forward(args) => {
                self.call_tool(KernelTool::Forward, forward_arguments(args, about))
            }
            ToolArguments::Trace(args) => self.call_tool(KernelTool::Trace, trace_arguments(args)),
            ToolArguments::Inspect(args) => {
                self.call_tool(KernelTool::Inspect, inspect_arguments(args))
            }
        }
    }
}

fn wake_arguments(args: &WakeArguments) -> Value {
    json!({ "about": args.about().as_str() })
}

fn ask_arguments(args: &AskArguments, about: &AboutId) -> Value {
    json!({
        "about": about.as_str(),
        "question": args.query().as_str(),
    })
}

fn near_arguments(args: &NearArguments, about: &AboutId) -> Value {
    json!({
        "about": about.as_str(),
        "around": { "ref": args.anchor().as_str() },
    })
}

fn goto_arguments(args: &GotoArguments, about: &AboutId) -> Value {
    json!({
        "about": about.as_str(),
        "at": cursor_to_anchor_json(args.cursor()),
    })
}

fn rewind_arguments(args: &RewindArguments, about: &AboutId) -> Value {
    json!({
        "about": about.as_str(),
        "from": {
            "key": args.cursor().key().as_str(),
            "anchor": args.cursor().anchor().as_str(),
        },
        "window": { "before_entries": args.window().as_usize() },
    })
}

fn forward_arguments(args: &ForwardArguments, about: &AboutId) -> Value {
    json!({
        "about": about.as_str(),
        "from": {
            "key": args.cursor().key().as_str(),
            "anchor": args.cursor().anchor().as_str(),
        },
        "window": { "after_entries": args.window().as_usize() },
    })
}

fn trace_arguments(args: &TraceArguments) -> Value {
    let mut body = json!({
        "from": args.from().as_str(),
        "page": args.page().as_usize(),
    });
    if let Some(to) = args.to() {
        body["to"] = Value::String(to.as_str().to_string());
    }
    body
}

fn inspect_arguments(args: &InspectArguments) -> Value {
    json!({ "ref": args.target().as_str() })
}

fn cursor_to_anchor_json(cursor: &Cursor) -> Value {
    match cursor {
        Cursor::Ref(c) => json!({ "ref": c.target().as_str() }),
        Cursor::Around(c) => json!({ "ref": c.anchor().as_str() }),
        Cursor::Temporal(c) => json!({
            "key": c.key().as_str(),
            "anchor": c.anchor().as_str(),
        }),
        Cursor::Trace(c) => json!({
            "from": c.from().as_str(),
            "to": c.to().as_str(),
        }),
    }
}

fn map_structured_content(
    tool: KernelTool,
    structured: &Value,
) -> Result<ToolOutcome, KmpClientError> {
    match tool {
        KernelTool::Ingest | KernelTool::WriteMemory => Err(KmpClientError::Protocol {
            tool: tool.as_str(),
            message: "write tool reached read-profile stdio executor".to_string(),
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

fn tool_response(outcome: ToolOutcome) -> Observation {
    let observed_refs = observed_refs_for(&outcome);
    Observation::ToolResponse {
        outcome,
        observed_refs,
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
