use std::sync::Arc;
use std::time::Instant;

use operator_runtime_domain::budget::session_budget::SessionBudget;
use operator_runtime_domain::session::execution_step::ExecutionStep;
use operator_runtime_domain::session::observation::Observation;
use operator_runtime_domain::session::operator_request::OperatorRequest;
use operator_runtime_domain::session::session_transcript::SessionTranscript;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::contract::action_contract_validator::ActionContractValidator;
use operator_shared_domain::contract::composite_action_contract_validator::CompositeActionContractValidator;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::task_family::TaskFamily;
use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
use operator_shared_domain::visible_state::context_coverage_deviation::ContextCoverageDeviation;
use operator_shared_domain::visible_state::visible_state::VisibleState;
use operator_synthetic_domain::calibration::calibration_subject::CalibrationSubject;

use crate::errors::runtime_error::RuntimeError;
use crate::ports::mcp_executor_port::McpExecutor;
use crate::ports::operator_policy_port::OperatorPolicy;
use crate::ports::session_event_sink_port::SessionEventSink;

/// Drives a full multi-step operator session: predict -> validate -> execute
/// one KMP move -> observe -> feed the observation back into the visible state
/// -> predict again, looping until the policy stops or escalates, the call
/// budget is exhausted, or a predicted action violates the contract.
///
/// This is what lets the operator progressively open a temporal/dimensional
/// window (for example growing `rewind`/`forward` bounds) across several KMP
/// calls until a period's evidence is fully covered. The loop is
/// policy-agnostic: it executes whatever bounded action the policy predicts and
/// accumulates the observed refs into the visible state; the decision to expand
/// vs. stop belongs to the policy, not to the loop.
///
/// Tool errors are recorded as steps and fed back to the policy (they are not
/// terminal), so the policy can adapt; the call budget bounds the loop. The
/// single-step use case is kept for read calibration and is deliberately not
/// retrofitted into a loop.
#[derive(Debug)]
pub struct RunOperatorSessionMultiStepUseCase {
    operator_policy: Arc<dyn OperatorPolicy>,
    mcp_executor: Arc<dyn McpExecutor>,
    validator: CompositeActionContractValidator,
    event_sink: Arc<dyn SessionEventSink>,
}

impl RunOperatorSessionMultiStepUseCase {
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

    pub fn execute(&self, request: &OperatorRequest) -> Result<SessionTranscript, RuntimeError> {
        let started_at = Instant::now();
        self.event_sink.on_request_received(request);

        let mut state = request.initial_visible_state().clone();
        let mut budget = request.initial_budget();
        let mut steps: Vec<ExecutionStep> = Vec::new();
        let mut deviation = ContextCoverageDeviation::initial();

        loop {
            let subject = build_subject(request, &state)?;
            let action = self.operator_policy.predict(&subject)?;
            self.event_sink.on_action_predicted(&action);

            // A tool call with no remaining budget ends the session as
            // budget-exhausted. This is checked before contract validation so it
            // takes precedence over the contract's budget specification, which
            // would otherwise report the same condition as a generic violation.
            if matches!(action, OperatorAction::ToolCall(_)) && !budget.allows_call() {
                return Ok(SessionTranscript::budget_exhausted(
                    request.session_id().clone(),
                    steps,
                    action,
                    state,
                    budget,
                    started_at.elapsed(),
                ));
            }

            if let Err(violations) =
                self.validator
                    .validate(&action, request.about(), request.mode(), &state)
            {
                return Ok(SessionTranscript::contract_violation(
                    request.session_id().clone(),
                    steps,
                    action,
                    violations,
                    state,
                    budget,
                    started_at.elapsed(),
                ));
            }

            if matches!(action, OperatorAction::Stop(_)) {
                return Ok(SessionTranscript::completed(
                    request.session_id().clone(),
                    steps,
                    action,
                    state,
                    budget,
                    started_at.elapsed(),
                ));
            }

            if matches!(action, OperatorAction::Escalate(_)) {
                return Ok(SessionTranscript::escalated(
                    request.session_id().clone(),
                    steps,
                    action,
                    state,
                    budget,
                    started_at.elapsed(),
                ));
            }

            // The remaining variant is a tool call with budget available.
            let observation = self.mcp_executor.execute(&action, request.about())?;
            self.event_sink.on_observation(&observation);
            budget = budget.try_consume_call()?;
            let observed = observed_refs(&observation);
            // Fold this step's intrinsic KMP signals into the online
            // context-coverage deviation, weighting the marginal-information
            // yield (refs this step newly surfaced). Tool errors carry no
            // signals and leave the estimate unchanged.
            let new_refs = count_new_refs(&state, &observed);
            if let Some(signals) = observation.signals() {
                deviation = deviation.observing(signals, new_refs);
            }
            let coverage_deviation = deviation.snapshot();
            steps.push(ExecutionStep::new(action, observation));
            state = state.observing(observed, budget_snapshot(budget), coverage_deviation);
        }
    }
}

fn build_subject(
    request: &OperatorRequest,
    state: &VisibleState,
) -> Result<CalibrationSubject, RuntimeError> {
    let task_family =
        TaskFamily::parse("runtime.multi_step").map_err(|err| RuntimeError::SubjectBuild {
            message: err.to_string(),
        })?;
    // `prepared_action` is a single-step copy mechanism; the multi-step loop
    // predicts every step from the evolving visible state, so it is never
    // forwarded into the subject.
    CalibrationSubject::new(
        request.about().clone(),
        request.mode(),
        task_family,
        request.goal().clone(),
        request.allowed_tools().clone(),
        state.clone(),
        None,
    )
    .map_err(|err| RuntimeError::SubjectBuild {
        message: err.to_string(),
    })
}

fn observed_refs(observation: &Observation) -> Vec<MemoryRef> {
    match observation {
        Observation::ToolResponse { observed_refs, .. } => observed_refs.clone(),
        Observation::ToolError { .. } | Observation::Terminal { .. } => Vec::new(),
    }
}

/// Marginal-information yield: how many of `observed` the operator did not
/// already know. Near-zero yield on consecutive expansions signals saturation.
fn count_new_refs(state: &VisibleState, observed: &[MemoryRef]) -> u32 {
    let count = observed
        .iter()
        .filter(|memory_ref| !state.knows_ref(memory_ref))
        .count();
    u32::try_from(count).unwrap_or(u32::MAX)
}

fn budget_snapshot(budget: SessionBudget) -> BudgetSnapshot {
    BudgetSnapshot::bounded(
        budget.calls_remaining() as usize,
        budget.tokens_remaining() as usize,
    )
}
