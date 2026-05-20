//! `ExecuteReplayUseCase` walks a slice of `ReplayPrediction` values,
//! dispatches each predicted action to the injected `KmpMcpClient`
//! port (or skips dispatch for `Stop` / `Escalate`) and assembles a
//! `ReplayReport`. Individual tool-call failures land in the report
//! as `ReplayOutcome::ToolCallFailed`; only an empty input is a hard
//! error.

use operator_replay_domain::execution::replay_execution::ReplayExecution;
use operator_replay_domain::outcome::replay_failure_reason::ReplayFailureReason;
use operator_replay_domain::outcome::replay_outcome::ReplayOutcome;
use operator_replay_domain::prediction::replay_prediction::ReplayPrediction;
use operator_replay_domain::report::replay_report::ReplayReport;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::action::tool_call_action::ToolCallAction;
use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
use operator_shared_domain::tool_outcomes::tool_outcome::ToolOutcome;

use crate::error::execute_replay_error::ExecuteReplayError;
use crate::error::kmp_client_error::KmpClientError;
use crate::ports::kmp_mcp_client::KmpMcpClient;

#[derive(Debug)]
pub struct ExecuteReplayUseCase<C: KmpMcpClient> {
    client: C,
}

impl<C: KmpMcpClient> ExecuteReplayUseCase<C> {
    pub fn new(client: C) -> Self {
        Self { client }
    }

    pub fn execute(
        &self,
        predictions: &[ReplayPrediction],
    ) -> Result<ReplayReport, ExecuteReplayError> {
        let mut executions = Vec::with_capacity(predictions.len());
        for prediction in predictions {
            let outcome = self.dispatch(prediction.action());
            executions.push(ReplayExecution::new(
                prediction.trajectory_id().clone(),
                prediction.action().clone(),
                outcome,
            ));
        }
        Ok(ReplayReport::new(executions)?)
    }

    fn dispatch(&self, action: &OperatorAction) -> ReplayOutcome {
        match action {
            OperatorAction::Stop(_) | OperatorAction::Escalate(_) => ReplayOutcome::NoToolCall,
            OperatorAction::ToolCall(call) => self.dispatch_tool_call(call),
        }
    }

    fn dispatch_tool_call(&self, call: &ToolCallAction) -> ReplayOutcome {
        let tool = call.tool();
        match call.arguments() {
            ToolArguments::Ingest(args) => match self.client.ingest(args) {
                Ok(o) => ReplayOutcome::ToolSucceeded(ToolOutcome::Ingest(o)),
                Err(e) => ReplayOutcome::ToolCallFailed {
                    tool,
                    reason: reason_from(&e),
                },
            },
            ToolArguments::Wake(args) => match self.client.wake(args) {
                Ok(o) => ReplayOutcome::ToolSucceeded(ToolOutcome::Wake(o)),
                Err(e) => ReplayOutcome::ToolCallFailed {
                    tool,
                    reason: reason_from(&e),
                },
            },
            ToolArguments::Ask(args) => match self.client.ask(args) {
                Ok(o) => ReplayOutcome::ToolSucceeded(ToolOutcome::Ask(o)),
                Err(e) => ReplayOutcome::ToolCallFailed {
                    tool,
                    reason: reason_from(&e),
                },
            },
            ToolArguments::Near(args) => match self.client.near(args) {
                Ok(o) => ReplayOutcome::ToolSucceeded(ToolOutcome::Near(o)),
                Err(e) => ReplayOutcome::ToolCallFailed {
                    tool,
                    reason: reason_from(&e),
                },
            },
            ToolArguments::Goto(args) => match self.client.goto(args) {
                Ok(o) => ReplayOutcome::ToolSucceeded(ToolOutcome::Goto(o)),
                Err(e) => ReplayOutcome::ToolCallFailed {
                    tool,
                    reason: reason_from(&e),
                },
            },
            ToolArguments::Rewind(args) => match self.client.rewind(args) {
                Ok(o) => ReplayOutcome::ToolSucceeded(ToolOutcome::Rewind(o)),
                Err(e) => ReplayOutcome::ToolCallFailed {
                    tool,
                    reason: reason_from(&e),
                },
            },
            ToolArguments::Forward(args) => match self.client.forward(args) {
                Ok(o) => ReplayOutcome::ToolSucceeded(ToolOutcome::Forward(o)),
                Err(e) => ReplayOutcome::ToolCallFailed {
                    tool,
                    reason: reason_from(&e),
                },
            },
            ToolArguments::Trace(args) => match self.client.trace(args) {
                Ok(o) => ReplayOutcome::ToolSucceeded(ToolOutcome::Trace(o)),
                Err(e) => ReplayOutcome::ToolCallFailed {
                    tool,
                    reason: reason_from(&e),
                },
            },
            ToolArguments::Inspect(args) => match self.client.inspect(args) {
                Ok(o) => ReplayOutcome::ToolSucceeded(ToolOutcome::Inspect(o)),
                Err(e) => ReplayOutcome::ToolCallFailed {
                    tool,
                    reason: reason_from(&e),
                },
            },
            ToolArguments::WriteMemory(args) => match self.client.write_memory(args) {
                Ok(o) => ReplayOutcome::ToolSucceeded(ToolOutcome::WriteMemory(o)),
                Err(e) => ReplayOutcome::ToolCallFailed {
                    tool,
                    reason: reason_from(&e),
                },
            },
        }
    }
}

/// Translates an adapter-layer `KmpClientError` (application crate)
/// into the domain-layer `ReplayFailureReason` (replay-domain crate)
/// that the report carries.
///
/// **There are three parallel enums for the same four failure
/// categories**:
/// - `KmpClientError`
///   (`operator-replay-application::error::kmp_client_error`) — the
///   adapter-facing error with full context (adapter name, tool name,
///   message). This is what implementations of `KmpMcpClient` raise.
/// - `ReplayFailureReason`
///   (`operator-replay-domain::outcome::replay_failure_reason`) — the
///   categorical reason recorded in a `ReplayReport`. No payload, just
///   the bucket.
/// - `FailureMode`
///   (`operator-replay-infra::adapters::in_memory_kmp_mcp_client`) —
///   test-fixture configuration for the in-memory stub.
///
/// The match below is exhaustive on `KmpClientError`, so adding a new
/// variant there is a compile-time error here. Add the corresponding
/// `ReplayFailureReason` variant (and the matching `FailureMode`
/// fixture branch) in lockstep when that happens. Each enum belongs to
/// its layer; the three-way split is intentional per the bounded
/// context rules in `docs/architecture/operator/00-principles.md`.
fn reason_from(error: &KmpClientError) -> ReplayFailureReason {
    match error {
        KmpClientError::Transport { .. } => ReplayFailureReason::Transport,
        KmpClientError::Protocol { .. } => ReplayFailureReason::Protocol,
        KmpClientError::InvalidArguments { .. } => ReplayFailureReason::InvalidArguments,
        KmpClientError::MalformedResponse { .. } => ReplayFailureReason::MalformedResponse,
    }
}
