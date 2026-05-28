use std::sync::Arc;
use std::time::Instant;

use operator_runtime_domain::session::operator_request::OperatorRequest;
use operator_runtime_domain::session::session_outcome::SessionOutcome;
use operator_runtime_domain::session::terminal_reason::TerminalReason;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::contract::action_contract_validator::ActionContractValidator;
use operator_shared_domain::contract::composite_action_contract_validator::CompositeActionContractValidator;
use operator_shared_domain::value_objects::task_family::TaskFamily;
use operator_synthetic_domain::calibration::calibration_subject::CalibrationSubject;
use operator_synthetic_domain::calibration::prepared_operator_action::PreparedOperatorAction;

use crate::errors::runtime_error::RuntimeError;
use crate::ports::mcp_executor_port::McpExecutor;
use crate::ports::operator_policy_port::OperatorPolicy;
use crate::ports::session_event_sink_port::SessionEventSink;

#[derive(Debug)]
pub struct RunOperatorSessionSingleStepUseCase {
    operator_policy: Arc<dyn OperatorPolicy>,
    mcp_executor: Arc<dyn McpExecutor>,
    validator: CompositeActionContractValidator,
    event_sink: Arc<dyn SessionEventSink>,
}

impl RunOperatorSessionSingleStepUseCase {
    pub fn new(
        operator_policy: Arc<dyn OperatorPolicy>,
        mcp_executor: Arc<dyn McpExecutor>,
        validator: CompositeActionContractValidator,
        event_sink: Arc<dyn SessionEventSink>,
    ) -> Self {
        Self {
            operator_policy,
            mcp_executor,
            validator,
            event_sink,
        }
    }

    pub fn execute(&self, request: &OperatorRequest) -> Result<SessionOutcome, RuntimeError> {
        let started_at = Instant::now();
        self.event_sink.on_request_received(request);

        let subject = build_subject_from_request(request)?;
        let action = self.operator_policy.predict(&subject)?;
        self.event_sink.on_action_predicted(&action);

        if let Err(violations) = self.validator.validate(
            &action,
            request.about(),
            request.mode(),
            request.initial_visible_state(),
        ) {
            let outcome = SessionOutcome::contract_violation(
                request,
                action,
                violations,
                started_at.elapsed(),
            );
            self.event_sink.on_session_complete(&outcome);
            return Ok(outcome);
        }

        match &action {
            OperatorAction::Stop(_) => {
                let outcome = SessionOutcome::terminal(
                    request,
                    action,
                    TerminalReason::Stop,
                    started_at.elapsed(),
                );
                self.event_sink.on_session_complete(&outcome);
                Ok(outcome)
            }
            OperatorAction::Escalate(_) => {
                let outcome = SessionOutcome::terminal(
                    request,
                    action,
                    TerminalReason::Escalate,
                    started_at.elapsed(),
                );
                self.event_sink.on_session_complete(&outcome);
                Ok(outcome)
            }
            OperatorAction::ToolCall(_) => self.execute_tool_call(request, action, started_at),
        }
    }

    fn execute_tool_call(
        &self,
        request: &OperatorRequest,
        action: OperatorAction,
        started_at: Instant,
    ) -> Result<SessionOutcome, RuntimeError> {
        if !request.initial_budget().allows_call() {
            let outcome = SessionOutcome::budget_exhausted(request, action, started_at.elapsed());
            self.event_sink.on_session_complete(&outcome);
            return Ok(outcome);
        }

        let observation = self.mcp_executor.execute(&action, request.about())?;
        self.event_sink.on_observation(&observation);
        let outcome =
            SessionOutcome::from_observation(request, action, observation, started_at.elapsed())?;
        self.event_sink.on_session_complete(&outcome);
        Ok(outcome)
    }
}

fn build_subject_from_request(
    request: &OperatorRequest,
) -> Result<CalibrationSubject, RuntimeError> {
    let task_family =
        TaskFamily::parse("runtime.single_step").map_err(|err| RuntimeError::SubjectBuild {
            message: err.to_string(),
        })?;
    let prepared_action = request
        .prepared_action()
        .map(|action| PreparedOperatorAction::new(action.clone()))
        .transpose()
        .map_err(|err| RuntimeError::SubjectBuild {
            message: err.to_string(),
        })?;
    CalibrationSubject::new(
        request.about().clone(),
        request.mode(),
        task_family,
        request.goal().clone(),
        request.allowed_tools().clone(),
        request.initial_visible_state().clone(),
        prepared_action,
    )
    .map_err(|err| RuntimeError::SubjectBuild {
        message: err.to_string(),
    })
}
