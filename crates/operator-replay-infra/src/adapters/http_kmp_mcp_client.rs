//! Real `KmpMcpClient` adapter that speaks MCP JSON-RPC 2.0 over HTTP
//! to a kernel MCP server. Per ADR 0011 there is no gRPC; the kernel
//! exposes operator-shaped tools (`kernel_wake`, `kernel_ask`, etc.)
//! through a JSON-RPC `tools/call` envelope, and this adapter speaks
//! that wire format directly.
//!
//! Known impedance gaps between our domain `ToolArguments` and the
//! kernel's MCP request schema (documented inline next to each tool):
//! - `kernel_ask` requires `about`; our `AskArguments` carries only
//!   `query`. We send the query as `question` and omit `about`. A
//!   real-kernel call may reject this with `InvalidArguments`; the
//!   domain change to introduce `about` lives in a follow-up PR.
//! - `kernel_near` / `goto` / `rewind` / `forward` use a temporal
//!   anchor shape (`time` / `sequence` / `ref`) that our domain
//!   models structurally. We send the memory ref under the anchor key
//!   each tool's schema names (`around`/`at`/`from`).
//!
//! The mapper unit tests fixture-validate the response shape; this
//! adapter's mock-server integration test exercises the request /
//! response round trip end to end.

use std::time::Duration;

use operator_replay_application::error::kmp_client_error::KmpClientError;
use operator_replay_application::ports::kmp_mcp_client::KmpMcpClient;
use operator_shared_domain::tool_arguments::ask_arguments::AskArguments;
use operator_shared_domain::tool_arguments::forward_arguments::ForwardArguments;
use operator_shared_domain::tool_arguments::goto_arguments::GotoArguments;
use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
use operator_shared_domain::tool_arguments::near_arguments::NearArguments;
use operator_shared_domain::tool_arguments::rewind_arguments::RewindArguments;
use operator_shared_domain::tool_arguments::trace_arguments::TraceArguments;
use operator_shared_domain::tool_arguments::wake_arguments::WakeArguments;
use operator_shared_domain::tool_arguments::write_memory_arguments::WriteMemoryArguments;
use operator_shared_domain::tool_outcomes::ask_outcome::AskOutcome;
use operator_shared_domain::tool_outcomes::forward_outcome::ForwardOutcome;
use operator_shared_domain::tool_outcomes::goto_outcome::GotoOutcome;
use operator_shared_domain::tool_outcomes::inspect_outcome::InspectOutcome;
use operator_shared_domain::tool_outcomes::near_outcome::NearOutcome;
use operator_shared_domain::tool_outcomes::rewind_outcome::RewindOutcome;
use operator_shared_domain::tool_outcomes::trace_outcome::TraceOutcome;
use operator_shared_domain::tool_outcomes::wake_outcome::WakeOutcome;
use operator_shared_domain::tool_outcomes::write_memory_outcome::WriteMemoryOutcome;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use serde_json::{Value, json};
use std::sync::atomic::{AtomicU64, Ordering};

use crate::jsonrpc::tools_call_request::ToolsCallRequest;
use crate::jsonrpc::tools_call_response::ToolsCallResponse;
use crate::mappers::ask_response_mapper::AskResponseMapper;
use crate::mappers::forward_response_mapper::ForwardResponseMapper;
use crate::mappers::goto_response_mapper::GotoResponseMapper;
use crate::mappers::inspect_response_mapper::InspectResponseMapper;
use crate::mappers::mapping_error::MappingError;
use crate::mappers::near_response_mapper::NearResponseMapper;
use crate::mappers::rewind_response_mapper::RewindResponseMapper;
use crate::mappers::trace_response_mapper::TraceResponseMapper;
use crate::mappers::wake_response_mapper::WakeResponseMapper;
use crate::mappers::write_memory_response_mapper::WriteMemoryResponseMapper;

const ADAPTER: &str = "http_kmp_mcp_client";
const DEFAULT_TIMEOUT_SECS: u64 = 30;

#[derive(Debug)]
pub struct HttpKmpMcpClient {
    endpoint: String,
    client: reqwest::blocking::Client,
    next_id: AtomicU64,
}

impl HttpKmpMcpClient {
    /// Construct an adapter that POSTs JSON-RPC to `endpoint`. The
    /// HTTP client is built with a 30-second timeout.
    pub fn new(endpoint: impl Into<String>) -> Result<Self, KmpClientError> {
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(DEFAULT_TIMEOUT_SECS))
            .build()
            .map_err(|err| KmpClientError::Transport {
                adapter: ADAPTER,
                tool: "<init>",
                message: format!("failed to build reqwest client: {err}"),
            })?;
        Ok(Self {
            endpoint: endpoint.into(),
            client,
            next_id: AtomicU64::new(1),
        })
    }

    /// Construct with a caller-built reqwest client (e.g., custom TLS,
    /// proxies, longer timeouts).
    pub fn with_client(endpoint: impl Into<String>, client: reqwest::blocking::Client) -> Self {
        Self {
            endpoint: endpoint.into(),
            client,
            next_id: AtomicU64::new(1),
        }
    }

    fn next_id(&self) -> u64 {
        self.next_id.fetch_add(1, Ordering::Relaxed)
    }

    fn call_tool(&self, tool: &'static str, arguments: Value) -> Result<Value, KmpClientError> {
        let request = ToolsCallRequest::new(self.next_id(), tool, arguments);
        let response = self.client.post(&self.endpoint).json(&request).send();
        let response = response.map_err(|err| KmpClientError::Transport {
            adapter: ADAPTER,
            tool,
            message: err.to_string(),
        })?;
        if !response.status().is_success() {
            return Err(KmpClientError::Protocol {
                tool,
                message: format!("server returned HTTP status {}", response.status()),
            });
        }
        let envelope: ToolsCallResponse =
            response.json().map_err(|err| KmpClientError::Protocol {
                tool,
                message: format!("invalid JSON-RPC envelope: {err}"),
            })?;
        if let Some(error) = envelope.error.as_ref() {
            return Err(KmpClientError::Protocol {
                tool,
                message: format!("JSON-RPC error {}: {}", error.code, error.message),
            });
        }
        envelope
            .structured_content()
            .map_err(|message| KmpClientError::MalformedResponse { tool, message })
    }
}

fn into_kmp_client_error(tool: &'static str, err: &MappingError) -> KmpClientError {
    KmpClientError::MalformedResponse {
        tool,
        message: err.to_string(),
    }
}

// Per-tool MCP request builders. They translate our domain
// `ToolArguments` into the JSON shape the kernel expects (NOT the
// trajectory wire shape from `operator-shared-contract`). The known
// impedance gaps are documented in the module header.

fn wake_request_arguments(args: &WakeArguments) -> Value {
    json!({ "about": args.about().as_str() })
}

fn ask_request_arguments(args: &AskArguments) -> Value {
    json!({ "question": args.query().as_str() })
}

fn near_request_arguments(args: &NearArguments) -> Value {
    json!({
        "about": args.anchor().as_str(),
        "around": { "ref": args.anchor().as_str() },
    })
}

fn goto_request_arguments(_args: &GotoArguments) -> Value {
    json!({ "at": {} })
}

fn rewind_request_arguments(args: &RewindArguments) -> Value {
    json!({
        "from": {
            "key": args.cursor().key().as_str(),
            "anchor": args.cursor().anchor().as_str(),
        },
        "window": { "before_entries": args.window().as_usize() },
    })
}

fn forward_request_arguments(args: &ForwardArguments) -> Value {
    json!({
        "from": {
            "key": args.cursor().key().as_str(),
            "anchor": args.cursor().anchor().as_str(),
        },
        "window": { "after_entries": args.window().as_usize() },
    })
}

fn trace_request_arguments(args: &TraceArguments) -> Value {
    let mut body = json!({
        "from": args.from().as_str(),
        "page": args.page().as_usize(),
    });
    if let Some(to) = args.to() {
        body["to"] = Value::String(to.as_str().to_string());
    }
    body
}

fn inspect_request_arguments(args: &InspectArguments) -> Value {
    json!({ "ref": args.target().as_str() })
}

fn write_memory_request_arguments(args: &WriteMemoryArguments) -> Value {
    json!({
        "summary": args.summary().as_str(),
        "body": args.body().as_str(),
        "related": args.related().iter().map(MemoryRef::as_str).collect::<Vec<_>>(),
    })
}

impl KmpMcpClient for HttpKmpMcpClient {
    fn wake(&self, args: &WakeArguments) -> Result<WakeOutcome, KmpClientError> {
        let structured = self.call_tool("kernel_wake", wake_request_arguments(args))?;
        WakeResponseMapper::to_outcome(&structured)
            .map_err(|e| into_kmp_client_error("kernel_wake", &e))
    }

    fn ask(&self, args: &AskArguments) -> Result<AskOutcome, KmpClientError> {
        let structured = self.call_tool("kernel_ask", ask_request_arguments(args))?;
        AskResponseMapper::to_outcome(&structured)
            .map_err(|e| into_kmp_client_error("kernel_ask", &e))
    }

    fn near(&self, args: &NearArguments) -> Result<NearOutcome, KmpClientError> {
        let structured = self.call_tool("kernel_near", near_request_arguments(args))?;
        NearResponseMapper::to_outcome(&structured)
            .map_err(|e| into_kmp_client_error("kernel_near", &e))
    }

    fn goto(&self, args: &GotoArguments) -> Result<GotoOutcome, KmpClientError> {
        let structured = self.call_tool("kernel_goto", goto_request_arguments(args))?;
        GotoResponseMapper::to_outcome(&structured)
            .map_err(|e| into_kmp_client_error("kernel_goto", &e))
    }

    fn rewind(&self, args: &RewindArguments) -> Result<RewindOutcome, KmpClientError> {
        let structured = self.call_tool("kernel_rewind", rewind_request_arguments(args))?;
        RewindResponseMapper::to_outcome(&structured)
            .map_err(|e| into_kmp_client_error("kernel_rewind", &e))
    }

    fn forward(&self, args: &ForwardArguments) -> Result<ForwardOutcome, KmpClientError> {
        let structured = self.call_tool("kernel_forward", forward_request_arguments(args))?;
        ForwardResponseMapper::to_outcome(&structured)
            .map_err(|e| into_kmp_client_error("kernel_forward", &e))
    }

    fn trace(&self, args: &TraceArguments) -> Result<TraceOutcome, KmpClientError> {
        let structured = self.call_tool("kernel_trace", trace_request_arguments(args))?;
        TraceResponseMapper::to_outcome(&structured)
            .map_err(|e| into_kmp_client_error("kernel_trace", &e))
    }

    fn inspect(&self, args: &InspectArguments) -> Result<InspectOutcome, KmpClientError> {
        let structured = self.call_tool("kernel_inspect", inspect_request_arguments(args))?;
        InspectResponseMapper::to_outcome(&structured)
            .map_err(|e| into_kmp_client_error("kernel_inspect", &e))
    }

    fn write_memory(
        &self,
        args: &WriteMemoryArguments,
    ) -> Result<WriteMemoryOutcome, KmpClientError> {
        let structured =
            self.call_tool("kernel_write_memory", write_memory_request_arguments(args))?;
        WriteMemoryResponseMapper::to_outcome(&structured)
            .map_err(|e| into_kmp_client_error("kernel_write_memory", &e))
    }
}
