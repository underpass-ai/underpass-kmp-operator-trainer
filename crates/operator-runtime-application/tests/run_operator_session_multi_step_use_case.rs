use std::collections::VecDeque;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use operator_runtime_application::errors::mcp_executor_error::McpExecutorError;
use operator_runtime_application::errors::operator_policy_error::OperatorPolicyError;
use operator_runtime_application::ports::mcp_executor_port::McpExecutor;
use operator_runtime_application::ports::operator_policy_port::OperatorPolicy;
use operator_runtime_application::ports::session_event_sink_port::SessionEventSink;
use operator_runtime_application::use_cases::run_operator_session_multi_step_use_case::RunOperatorSessionMultiStepUseCase;
use operator_runtime_domain::budget::session_budget::SessionBudget;
use operator_runtime_domain::session::observation::Observation;
use operator_runtime_domain::session::observation_error_code::ObservationErrorCode;
use operator_runtime_domain::session::operator_request::OperatorRequest;
use operator_runtime_domain::session::operator_session_id::OperatorSessionId;
use operator_runtime_domain::session::outcome_class::OutcomeClass;
use operator_runtime_domain::session::session_outcome::SessionOutcome;
use operator_shared_domain::action::escalate_action::EscalateAction;
use operator_shared_domain::action::escalate_reason::EscalateReason;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::action::stop_action::StopAction;
use operator_shared_domain::action::stop_reason::StopReason;
use operator_shared_domain::action::tool_call_action::ToolCallAction;
use operator_shared_domain::contract::composite_action_contract_validator::CompositeActionContractValidator;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::mode::allowed_tools::AllowedTools;
use operator_shared_domain::mode::operator_mode::OperatorMode;
use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
use operator_shared_domain::tool_arguments::write_memory_arguments::WriteMemoryArguments;
use operator_shared_domain::tool_outcomes::inspect_outcome::InspectOutcome;
use operator_shared_domain::tool_outcomes::tool_outcome::ToolOutcome;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::model_id::ModelId;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;
use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
use operator_shared_domain::visible_state::visible_state::VisibleState;
use operator_synthetic_domain::calibration::calibration_subject::CalibrationSubject;

#[derive(Debug)]
struct ScriptedOperatorPolicy {
    actions: Mutex<VecDeque<OperatorAction>>,
    fallback: OperatorAction,
}

impl ScriptedOperatorPolicy {
    fn new(actions: Vec<OperatorAction>, fallback: OperatorAction) -> Self {
        Self {
            actions: Mutex::new(actions.into()),
            fallback,
        }
    }
}

impl OperatorPolicy for ScriptedOperatorPolicy {
    fn predict(
        &self,
        _subject: &CalibrationSubject,
    ) -> Result<OperatorAction, OperatorPolicyError> {
        let mut queue = self.actions.lock().unwrap();
        Ok(queue.pop_front().unwrap_or_else(|| self.fallback.clone()))
    }
}

#[derive(Debug)]
struct SeqMcpExecutor {
    observations: Mutex<VecDeque<Observation>>,
    fallback: Observation,
    calls: Arc<AtomicUsize>,
}

impl McpExecutor for SeqMcpExecutor {
    fn execute(
        &self,
        _action: &OperatorAction,
        _about: &AboutId,
    ) -> Result<Observation, McpExecutorError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        let mut queue = self.observations.lock().unwrap();
        Ok(queue.pop_front().unwrap_or_else(|| self.fallback.clone()))
    }
}

#[derive(Debug)]
struct NoopSessionEventSink;

impl SessionEventSink for NoopSessionEventSink {
    fn on_request_received(&self, _request: &OperatorRequest) {}
    fn on_action_predicted(&self, _action: &OperatorAction) {}
    fn on_observation(&self, _observation: &Observation) {}
    fn on_session_complete(&self, _outcome: &SessionOutcome) {}
}

fn text(value: &str) -> NonEmptyString {
    NonEmptyString::parse(value, "runtime_multi_step_test").unwrap()
}

fn memory_ref(value: &str) -> MemoryRef {
    MemoryRef::parse(value).unwrap()
}

fn inspect_action(target: &str) -> OperatorAction {
    OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
        InspectArguments::new(memory_ref(target)),
    )))
}

fn write_action() -> OperatorAction {
    OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::WriteMemory(
        WriteMemoryArguments::new("summary", "body", vec![]).unwrap(),
    )))
}

fn stop_action() -> OperatorAction {
    OperatorAction::Stop(StopAction::new(StopReason::AnswerReady, None, vec![]).unwrap())
}

fn escalate_action() -> OperatorAction {
    OperatorAction::Escalate(EscalateAction::new(
        EscalateReason::BeyondCapability,
        ModelId::parse("claude-opus-4-7").unwrap(),
    ))
}

fn success_observation(target: &str) -> Observation {
    Observation::ToolResponse {
        outcome: ToolOutcome::Inspect(InspectOutcome::new(
            text("inspect"),
            memory_ref(target),
            text("node"),
            vec![],
            vec![],
        )),
        observed_refs: vec![memory_ref(target)],
        signals:
            operator_shared_domain::visible_state::navigation_signals::NavigationSignals::empty(),
    }
}

fn error_observation() -> Observation {
    Observation::ToolError {
        code: ObservationErrorCode::parse("kernel_rejected").unwrap(),
        message: "kernel rejected request".to_string(),
    }
}

fn request_with_budget(calls_remaining: u32) -> OperatorRequest {
    OperatorRequest::new(
        OperatorSessionId::parse("session:runtime-multi").unwrap(),
        TrajectoryGoal::parse("Count events across the period.").unwrap(),
        VisibleState::assemble(
            [memory_ref("node:1")],
            [],
            None,
            BudgetSnapshot::bounded(1, 4096),
        ),
        OperatorMode::Read,
        AllowedTools::for_mode(OperatorMode::Read),
        SessionBudget::new(calls_remaining, 4096),
        AboutId::parse("about:test").unwrap(),
        None,
    )
    .unwrap()
}

fn use_case(
    actions: Vec<OperatorAction>,
    observations: Vec<Observation>,
    calls: Arc<AtomicUsize>,
) -> RunOperatorSessionMultiStepUseCase {
    RunOperatorSessionMultiStepUseCase::new(
        Arc::new(ScriptedOperatorPolicy::new(actions, stop_action())),
        Arc::new(SeqMcpExecutor {
            observations: Mutex::new(observations.into()),
            fallback: success_observation("node:9"),
            calls,
        }),
        CompositeActionContractValidator::default_strict(),
        Arc::new(NoopSessionEventSink),
    )
}

#[test]
fn loops_through_tool_calls_until_stop_accumulating_refs_and_consuming_budget() {
    let calls = Arc::new(AtomicUsize::new(0));
    let use_case = use_case(
        // Each inspect targets a ref already known: node:1 from the initial
        // state, then node:2 once the first observation surfaces it. This is
        // the loop's point — observations feed the visible state the next
        // prediction sees.
        vec![
            inspect_action("node:1"),
            inspect_action("node:2"),
            stop_action(),
        ],
        vec![success_observation("node:2"), success_observation("node:3")],
        calls.clone(),
    );

    let transcript = use_case
        .execute(&request_with_budget(5))
        .expect("multi-step session succeeds");

    assert!(matches!(
        transcript.outcome_class(),
        OutcomeClass::Completed
    ));
    assert!(matches!(
        transcript.terminal_action(),
        OperatorAction::Stop(_)
    ));
    assert_eq!(transcript.step_count(), 2);
    assert_eq!(calls.load(Ordering::Relaxed), 2);
    assert_eq!(transcript.final_budget().calls_remaining(), 3);
    // The observations fed back into the visible state the policy perceives.
    let final_state = transcript.final_visible_state();
    assert!(final_state.knows_ref(&memory_ref("node:1")));
    assert!(final_state.knows_ref(&memory_ref("node:2")));
    assert!(final_state.knows_ref(&memory_ref("node:3")));
    assert!(transcript.elapsed_ms() < u64::MAX);
}

#[test]
fn stops_when_call_budget_is_exhausted_mid_loop() {
    let calls = Arc::new(AtomicUsize::new(0));
    let use_case = use_case(
        vec![
            inspect_action("node:1"),
            inspect_action("node:2"),
            inspect_action("node:3"),
        ],
        vec![success_observation("node:2"), success_observation("node:3")],
        calls.clone(),
    );

    let transcript = use_case
        .execute(&request_with_budget(2))
        .expect("budget exhaustion is a transcript, not an error");

    assert!(matches!(
        transcript.outcome_class(),
        OutcomeClass::BudgetExhausted
    ));
    assert_eq!(transcript.step_count(), 2);
    assert_eq!(calls.load(Ordering::Relaxed), 2);
    assert_eq!(transcript.final_budget().calls_remaining(), 0);
}

#[test]
fn escalates_when_policy_escalates() {
    let calls = Arc::new(AtomicUsize::new(0));
    let use_case = use_case(
        vec![inspect_action("node:1"), escalate_action()],
        vec![success_observation("node:2")],
        calls.clone(),
    );

    let transcript = use_case
        .execute(&request_with_budget(5))
        .expect("escalation is a transcript");

    assert!(matches!(
        transcript.outcome_class(),
        OutcomeClass::Escalated
    ));
    assert!(matches!(
        transcript.terminal_action(),
        OperatorAction::Escalate(_)
    ));
    assert_eq!(transcript.step_count(), 1);
    assert_eq!(calls.load(Ordering::Relaxed), 1);
}

#[test]
fn first_action_stop_completes_with_no_steps() {
    let calls = Arc::new(AtomicUsize::new(0));
    let use_case = use_case(vec![stop_action()], vec![], calls.clone());

    let transcript = use_case
        .execute(&request_with_budget(5))
        .expect("immediate stop is a transcript");

    assert!(matches!(
        transcript.outcome_class(),
        OutcomeClass::Completed
    ));
    assert_eq!(transcript.step_count(), 0);
    assert_eq!(calls.load(Ordering::Relaxed), 0);
    assert!(transcript.steps().is_empty());
}

#[test]
fn contract_violation_ends_session_without_executing() {
    let calls = Arc::new(AtomicUsize::new(0));
    // write tool is not allowed in Read mode: the validator rejects it.
    let use_case = use_case(vec![write_action()], vec![], calls.clone());

    let transcript = use_case
        .execute(&request_with_budget(5))
        .expect("contract violation is a transcript");

    assert!(matches!(
        transcript.outcome_class(),
        OutcomeClass::ContractViolation { .. }
    ));
    assert_eq!(transcript.step_count(), 0);
    assert_eq!(calls.load(Ordering::Relaxed), 0);
}

#[test]
fn tool_error_is_recorded_and_fed_back_not_terminal() {
    let calls = Arc::new(AtomicUsize::new(0));
    let use_case = use_case(
        // Re-inspect node:1 (always known) so the second step is valid; the
        // first observation is a tool error that surfaces no refs.
        vec![
            inspect_action("node:1"),
            inspect_action("node:1"),
            stop_action(),
        ],
        vec![error_observation(), success_observation("node:2")],
        calls.clone(),
    );

    let transcript = use_case
        .execute(&request_with_budget(5))
        .expect("tool error does not abort the session");

    // The session continued past the tool error and stopped normally.
    assert!(matches!(
        transcript.outcome_class(),
        OutcomeClass::Completed
    ));
    assert_eq!(transcript.step_count(), 2);
    assert_eq!(calls.load(Ordering::Relaxed), 2);
    // The first recorded step carries the tool error observation.
    assert!(transcript.steps()[0].observation().is_tool_error());
}
