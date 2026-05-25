use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use operator_runtime_application::ports::session_event_sink_port::SessionEventSink;
use operator_runtime_domain::session::observation::Observation;
use operator_runtime_domain::session::operator_request::OperatorRequest;
use operator_runtime_domain::session::session_outcome::SessionOutcome;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::tool::kernel_tool::KernelTool;
use serde_json::json;

#[derive(Debug)]
pub struct JsonlSessionEventSink {
    file: Mutex<File>,
    operator_endpoint: String,
    mcp_endpoint: String,
    adapter_sha: String,
}

impl JsonlSessionEventSink {
    pub fn new(
        path: &Path,
        operator_endpoint: impl Into<String>,
        mcp_endpoint: impl Into<String>,
        adapter_sha: impl Into<String>,
    ) -> std::io::Result<Self> {
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        Ok(Self {
            file: Mutex::new(file),
            operator_endpoint: operator_endpoint.into(),
            mcp_endpoint: mcp_endpoint.into(),
            adapter_sha: adapter_sha.into(),
        })
    }

    fn write_event(&self, value: &serde_json::Value) {
        if let Ok(mut file) = self.file.lock() {
            let _ = writeln!(file, "{value}");
        }
    }
}

impl SessionEventSink for JsonlSessionEventSink {
    fn on_request_received(&self, request: &OperatorRequest) {
        self.write_event(&json!({
            "event": "request_received",
            "timestamp_unix_ms": timestamp_unix_ms(),
            "session_id": request.session_id().as_str(),
            "mode": request.mode().as_str(),
            "about": request.about().as_str(),
            "goal": request.goal().as_str(),
            "budget": {
                "calls_remaining": request.initial_budget().calls_remaining(),
                "tokens_remaining": request.initial_budget().tokens_remaining()
            },
            "operator_endpoint": self.operator_endpoint.as_str(),
            "mcp_endpoint": self.mcp_endpoint.as_str(),
            "adapter_sha": self.adapter_sha.as_str()
        }));
    }

    fn on_action_predicted(&self, action: &OperatorAction) {
        self.write_event(&json!({
            "event": "action_predicted",
            "timestamp_unix_ms": timestamp_unix_ms(),
            "kind": action.kind().as_str(),
            "tool": action.tool().map(KernelTool::as_str)
        }));
    }

    fn on_observation(&self, observation: &Observation) {
        self.write_event(&json!({
            "event": "observation",
            "timestamp_unix_ms": timestamp_unix_ms(),
            "class": match observation {
                Observation::ToolResponse { .. } => "tool_response",
                Observation::ToolError { .. } => "tool_error",
                Observation::Terminal { .. } => "terminal",
            }
        }));
    }

    fn on_session_complete(&self, outcome: &SessionOutcome) {
        self.write_event(&json!({
            "event": "session_complete",
            "timestamp_unix_ms": timestamp_unix_ms(),
            "session_id": outcome.session_id().as_str(),
            "outcome": {
                "class": outcome.outcome_class().as_str(),
                "elapsed_ms": outcome.elapsed_ms(),
                "final_budget": {
                    "calls_remaining": outcome.final_budget().calls_remaining(),
                    "tokens_remaining": outcome.final_budget().tokens_remaining()
                }
            },
            "predicted_action": {
                "kind": outcome.predicted_action().kind().as_str(),
                "tool": outcome.predicted_action().tool().map(KernelTool::as_str)
            },
            "operator_endpoint": self.operator_endpoint.as_str(),
            "mcp_endpoint": self.mcp_endpoint.as_str(),
            "adapter_sha": self.adapter_sha.as_str()
        }));
    }
}

fn timestamp_unix_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_millis())
}
