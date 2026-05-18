//! Stub `KmpMcpClient` for tests. Three flavours:
//!
//! - `InMemoryKmpMcpClient::ok()` returns canned successful outcomes
//!   for every tool. Inputs are ignored beyond compile-time typing.
//! - `InMemoryKmpMcpClient::always_failing(reason)` returns a
//!   `KmpClientError` derived from the supplied failure mode for
//!   every tool.
//! - `InMemoryKmpMcpClient::failing_tools(tools, mode)` lets a test
//!   pin which tools fail and which succeed in the same run; the rest
//!   return the canned successful outcome.
//!
//! The real MCP JSON-RPC client lives in a future PR. This stub keeps
//! the application use case testable end-to-end without a running
//! kernel.

use std::collections::BTreeSet;

use operator_replay_application::error::kmp_client_error::KmpClientError;
use operator_replay_application::ports::kmp_mcp_client::KmpMcpClient;
use operator_shared_domain::tool::kernel_tool::KernelTool;
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
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureMode {
    Transport,
    Protocol,
    InvalidArguments,
    MalformedResponse,
}

#[derive(Debug)]
pub struct InMemoryKmpMcpClient {
    failing_tools: BTreeSet<KernelTool>,
    failure_mode: FailureMode,
}

impl InMemoryKmpMcpClient {
    /// Every tool returns the canned successful outcome.
    pub fn ok() -> Self {
        Self {
            failing_tools: BTreeSet::new(),
            // failure_mode is irrelevant when failing_tools is empty;
            // any default works.
            failure_mode: FailureMode::Transport,
        }
    }

    /// Every tool returns the configured failure.
    pub fn always_failing(failure_mode: FailureMode) -> Self {
        Self {
            failing_tools: KernelTool::ALL.iter().copied().collect(),
            failure_mode,
        }
    }

    /// Only the listed tools fail (with the configured mode); the
    /// rest return the canned successful outcome.
    pub fn failing_tools(
        tools: impl IntoIterator<Item = KernelTool>,
        failure_mode: FailureMode,
    ) -> Self {
        Self {
            failing_tools: tools.into_iter().collect(),
            failure_mode,
        }
    }

    fn should_fail(&self, tool: KernelTool) -> bool {
        self.failing_tools.contains(&tool)
    }

    fn failure_for(&self, tool: &'static str) -> KmpClientError {
        let message = "stub adapter configured to fail this tool".to_string();
        match self.failure_mode {
            FailureMode::Transport => KmpClientError::Transport {
                adapter: "in_memory_kmp_mcp_client",
                tool,
                message,
            },
            FailureMode::Protocol => KmpClientError::Protocol { tool, message },
            FailureMode::InvalidArguments => KmpClientError::InvalidArguments { tool, message },
            FailureMode::MalformedResponse => KmpClientError::MalformedResponse { tool, message },
        }
    }
}

fn summary(text: &str) -> NonEmptyString {
    NonEmptyString::parse(text, "in_memory_kmp_mcp_client.summary").expect("static")
}

fn surfaced_ref() -> MemoryRef {
    MemoryRef::parse("node:stub").expect("static")
}

impl KmpMcpClient for InMemoryKmpMcpClient {
    fn wake(&self, _args: &WakeArguments) -> Result<WakeOutcome, KmpClientError> {
        if self.should_fail(KernelTool::Wake) {
            return Err(self.failure_for("kernel_wake"));
        }
        Ok(WakeOutcome::new(summary("stub wake"), vec![surfaced_ref()]))
    }

    fn ask(&self, _args: &AskArguments) -> Result<AskOutcome, KmpClientError> {
        if self.should_fail(KernelTool::Ask) {
            return Err(self.failure_for("kernel_ask"));
        }
        Ok(AskOutcome::new(
            summary("stub ask"),
            Some(summary("stub answer")),
            vec![surfaced_ref()],
        ))
    }

    fn near(&self, _args: &NearArguments) -> Result<NearOutcome, KmpClientError> {
        if self.should_fail(KernelTool::Near) {
            return Err(self.failure_for("kernel_near"));
        }
        Ok(NearOutcome::new(summary("stub near"), vec![surfaced_ref()]))
    }

    fn goto(&self, _args: &GotoArguments) -> Result<GotoOutcome, KmpClientError> {
        if self.should_fail(KernelTool::Goto) {
            return Err(self.failure_for("kernel_goto"));
        }
        Ok(GotoOutcome::new(summary("stub goto"), vec![surfaced_ref()]))
    }

    fn rewind(&self, _args: &RewindArguments) -> Result<RewindOutcome, KmpClientError> {
        if self.should_fail(KernelTool::Rewind) {
            return Err(self.failure_for("kernel_rewind"));
        }
        Ok(RewindOutcome::new(
            summary("stub rewind"),
            vec![surfaced_ref()],
        ))
    }

    fn forward(&self, _args: &ForwardArguments) -> Result<ForwardOutcome, KmpClientError> {
        if self.should_fail(KernelTool::Forward) {
            return Err(self.failure_for("kernel_forward"));
        }
        Ok(ForwardOutcome::new(
            summary("stub forward"),
            vec![surfaced_ref()],
        ))
    }

    fn trace(&self, _args: &TraceArguments) -> Result<TraceOutcome, KmpClientError> {
        if self.should_fail(KernelTool::Trace) {
            return Err(self.failure_for("kernel_trace"));
        }
        Ok(TraceOutcome::new(
            summary("stub trace"),
            vec![surfaced_ref()],
        ))
    }

    fn inspect(&self, _args: &InspectArguments) -> Result<InspectOutcome, KmpClientError> {
        if self.should_fail(KernelTool::Inspect) {
            return Err(self.failure_for("kernel_inspect"));
        }
        Ok(InspectOutcome::new(
            summary("stub inspect"),
            surfaced_ref(),
            summary("claim"),
            vec![],
            vec![],
        ))
    }

    fn write_memory(
        &self,
        _args: &WriteMemoryArguments,
    ) -> Result<WriteMemoryOutcome, KmpClientError> {
        if self.should_fail(KernelTool::WriteMemory) {
            return Err(self.failure_for("kernel_write_memory"));
        }
        Ok(WriteMemoryOutcome::new(
            summary("stub write"),
            true,
            false,
            vec![surfaced_ref()],
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::ids::about_id::AboutId;

    #[test]
    fn ok_mode_returns_canned_outcome() {
        let client = InMemoryKmpMcpClient::ok();
        let result = client.wake(&WakeArguments::new(AboutId::parse("about:1").unwrap()));
        assert!(result.is_ok());
    }

    #[test]
    fn always_failing_returns_configured_failure_for_every_tool() {
        let client = InMemoryKmpMcpClient::always_failing(FailureMode::Transport);
        let err = client
            .wake(&WakeArguments::new(AboutId::parse("about:1").unwrap()))
            .unwrap_err();
        assert!(matches!(err, KmpClientError::Transport { tool, .. } if tool == "kernel_wake"));
    }

    #[test]
    fn failing_tools_succeeds_for_unlisted_tools() {
        let client =
            InMemoryKmpMcpClient::failing_tools([KernelTool::Inspect], FailureMode::Protocol);
        // Wake is not in the failing set, so it succeeds.
        assert!(
            client
                .wake(&WakeArguments::new(AboutId::parse("about:1").unwrap()))
                .is_ok()
        );
        // Inspect is in the set, so it fails with Protocol.
        let err = client
            .inspect(&InspectArguments::new(MemoryRef::parse("node:1").unwrap()))
            .unwrap_err();
        assert!(matches!(err, KmpClientError::Protocol { tool, .. } if tool == "kernel_inspect"));
    }
}
