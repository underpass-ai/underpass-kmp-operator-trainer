use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use operator_runtime_application::errors::mcp_executor_error::McpExecutorError;
use operator_runtime_application::errors::operator_policy_error::OperatorPolicyError;
use operator_runtime_application::errors::runtime_error::RuntimeError;
use operator_runtime_application::ports::mcp_executor_port::McpExecutor;
use operator_runtime_application::ports::operator_policy_port::OperatorPolicy;
use operator_runtime_application::ports::session_event_sink_port::SessionEventSink;
use operator_runtime_application::use_cases::run_operator_session_single_step_use_case::RunOperatorSessionSingleStepUseCase;
use operator_runtime_domain::budget::session_budget::SessionBudget;
use operator_runtime_domain::session::observation::Observation;
use operator_runtime_domain::session::observation_error_code::ObservationErrorCode;
use operator_runtime_domain::session::operator_request::OperatorRequest;
use operator_runtime_domain::session::operator_session_id::OperatorSessionId;
use operator_runtime_domain::session::outcome_class::OutcomeClass;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::action::stop_action::StopAction;
use operator_shared_domain::action::stop_reason::StopReason;
use operator_shared_domain::action::tool_call_action::ToolCallAction;
use operator_shared_domain::contract::composite_action_contract_validator::CompositeActionContractValidator;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::mode::allowed_tools::AllowedTools;
use operator_shared_domain::mode::operator_mode::OperatorMode;
use operator_shared_domain::tool_arguments::ask_arguments::AskArguments;
use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
use operator_shared_domain::tool_arguments::write_memory_arguments::WriteMemoryArguments;
use operator_shared_domain::tool_outcomes::inspect_outcome::InspectOutcome;
use operator_shared_domain::tool_outcomes::tool_outcome::ToolOutcome;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;
use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
use operator_shared_domain::visible_state::visible_state::VisibleState;
use operator_synthetic_domain::calibration::calibration_subject::CalibrationSubject;

#[derive(Debug)]
struct MockOperatorPolicy {
    canned: OperatorAction,
}

impl OperatorPolicy for MockOperatorPolicy {
    fn predict(
        &self,
        _subject: &CalibrationSubject,
    ) -> Result<OperatorAction, OperatorPolicyError> {
        Ok(self.canned.clone())
    }
}

#[derive(Debug)]
struct MockMcpExecutor {
    canned: Observation,
    calls: Arc<AtomicUsize>,
}

impl McpExecutor for MockMcpExecutor {
    fn execute(
        &self,
        _action: &OperatorAction,
        _about: &AboutId,
    ) -> Result<Observation, McpExecutorError> {
        self.calls.fetch_add(1, Ordering::Relaxed);
        Ok(self.canned.clone())
    }
}

#[derive(Debug)]
struct NoopSessionEventSink;

impl SessionEventSink for NoopSessionEventSink {
    fn on_request_received(&self, _request: &OperatorRequest) {}

    fn on_action_predicted(&self, _action: &OperatorAction) {}

    fn on_observation(&self, _observation: &Observation) {}

    fn on_session_complete(
        &self,
        _outcome: &operator_runtime_domain::session::session_outcome::SessionOutcome,
    ) {
    }
}

fn text(value: &str) -> NonEmptyString {
    NonEmptyString::parse(value, "runtime_application_test").unwrap()
}

fn target() -> MemoryRef {
    MemoryRef::parse("node:1").unwrap()
}

fn inspect_action() -> OperatorAction {
    OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
        InspectArguments::new(target()),
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

fn success_observation() -> Observation {
    Observation::ToolResponse {
        outcome: ToolOutcome::Inspect(InspectOutcome::new(
            text("inspect"),
            target(),
            text("node"),
            vec![],
            vec![],
        )),
        observed_refs: vec![target()],
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
        OperatorSessionId::parse("session:runtime").unwrap(),
        TrajectoryGoal::parse("Inspect node one.").unwrap(),
        VisibleState::assemble([target()], [], None, BudgetSnapshot::bounded(1, 4096)),
        OperatorMode::Read,
        AllowedTools::for_mode(OperatorMode::Read),
        SessionBudget::new(calls_remaining, 4096),
        AboutId::parse("about:test").unwrap(),
        None,
    )
    .unwrap()
}

fn ask_action(query: &str) -> OperatorAction {
    OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Ask(
        AskArguments::new(query).unwrap(),
    )))
}

fn request_with_prepared_action(
    mode: OperatorMode,
    allowed_tools: AllowedTools,
    prepared_action: Option<OperatorAction>,
) -> OperatorRequest {
    OperatorRequest::new(
        OperatorSessionId::parse("session:runtime-pa").unwrap(),
        TrajectoryGoal::parse("Inspect node one.").unwrap(),
        VisibleState::assemble([target()], [], None, BudgetSnapshot::bounded(1, 4096)),
        mode,
        allowed_tools,
        SessionBudget::new(1, 4096),
        AboutId::parse("about:test").unwrap(),
        prepared_action,
    )
    .unwrap()
}

#[derive(Debug)]
struct CapturingOperatorPolicy {
    captured: Arc<Mutex<Option<CalibrationSubject>>>,
    canned: OperatorAction,
}

impl OperatorPolicy for CapturingOperatorPolicy {
    fn predict(&self, subject: &CalibrationSubject) -> Result<OperatorAction, OperatorPolicyError> {
        *self.captured.lock().unwrap() = Some(subject.clone());
        Ok(self.canned.clone())
    }
}

fn capturing_use_case(
    canned: OperatorAction,
    captured: Arc<Mutex<Option<CalibrationSubject>>>,
) -> RunOperatorSessionSingleStepUseCase {
    RunOperatorSessionSingleStepUseCase::new(
        Arc::new(CapturingOperatorPolicy { captured, canned }),
        Arc::new(MockMcpExecutor {
            canned: success_observation(),
            calls: Arc::new(AtomicUsize::new(0)),
        }),
        CompositeActionContractValidator::default_strict(),
        Arc::new(NoopSessionEventSink),
    )
}

fn use_case(
    action: OperatorAction,
    observation: Observation,
    calls: Arc<AtomicUsize>,
) -> RunOperatorSessionSingleStepUseCase {
    RunOperatorSessionSingleStepUseCase::new(
        Arc::new(MockOperatorPolicy { canned: action }),
        Arc::new(MockMcpExecutor {
            canned: observation,
            calls,
        }),
        CompositeActionContractValidator::default_strict(),
        Arc::new(NoopSessionEventSink),
    )
}

#[test]
fn happy_path_completed_outcome() {
    let calls = Arc::new(AtomicUsize::new(0));
    let use_case = use_case(inspect_action(), success_observation(), calls.clone());

    let outcome = use_case
        .execute(&request_with_budget(1))
        .expect("use case succeeds");

    assert!(matches!(outcome.outcome_class(), OutcomeClass::Completed));
    assert_eq!(outcome.final_budget().calls_remaining(), 0);
    assert_eq!(calls.load(Ordering::Relaxed), 1);
}

#[test]
fn contract_violation_outcome_skips_mcp_execution() {
    let calls = Arc::new(AtomicUsize::new(0));
    let use_case = use_case(write_action(), success_observation(), calls.clone());

    let outcome = use_case
        .execute(&request_with_budget(1))
        .expect("contract violation is an outcome");

    assert!(matches!(
        outcome.outcome_class(),
        OutcomeClass::ContractViolation { .. }
    ));
    assert_eq!(calls.load(Ordering::Relaxed), 0);
}

#[test]
fn budget_exhausted_outcome_skips_mcp_execution() {
    let calls = Arc::new(AtomicUsize::new(0));
    let use_case = use_case(inspect_action(), success_observation(), calls.clone());

    let outcome = use_case
        .execute(&request_with_budget(0))
        .expect("budget exhausted is an outcome");

    assert!(matches!(
        outcome.outcome_class(),
        OutcomeClass::BudgetExhausted
    ));
    assert_eq!(calls.load(Ordering::Relaxed), 0);
}

#[test]
fn terminal_stop_outcome_skips_mcp_execution() {
    let calls = Arc::new(AtomicUsize::new(0));
    let use_case = use_case(stop_action(), success_observation(), calls.clone());

    let outcome = use_case
        .execute(&request_with_budget(0))
        .expect("terminal stop is an outcome");

    assert!(matches!(outcome.outcome_class(), OutcomeClass::Completed));
    assert!(matches!(
        outcome.observation(),
        Some(Observation::Terminal { .. })
    ));
    assert_eq!(calls.load(Ordering::Relaxed), 0);
}

#[test]
fn mcp_failure_outcome_propagates_error_code() {
    let calls = Arc::new(AtomicUsize::new(0));
    let use_case = use_case(inspect_action(), error_observation(), calls.clone());

    let outcome = use_case
        .execute(&request_with_budget(1))
        .expect("tool error is an outcome");

    assert!(matches!(
        outcome.outcome_class(),
        OutcomeClass::McpExecutionFailure { .. }
    ));
    assert_eq!(calls.load(Ordering::Relaxed), 1);
}

// --- Provisional prepared_action pass-through ----------------------------------
// These tests pin the contract used by v8.1.5 ask copy probes: when the
// OperatorRequest carries a prepared_action, the runtime forwards it into the
// CalibrationSubject that the policy receives; when it does not, the runtime
// preserves the historical None behavior. The runtime is NOT expected to
// synthesize a prepared_action.

#[test]
fn subject_builder_includes_prepared_action_when_present() {
    let pa = ask_action("What is the active constraint for the migration plan?");
    let captured = Arc::new(Mutex::new(None));
    let use_case = capturing_use_case(inspect_action(), Arc::clone(&captured));

    let request = request_with_prepared_action(
        OperatorMode::Read,
        AllowedTools::for_mode(OperatorMode::Read),
        Some(pa.clone()),
    );
    let _outcome = use_case.execute(&request).expect("predict succeeds");

    let subject = captured.lock().unwrap();
    let subject = subject.as_ref().expect("policy received subject");
    let prepared = subject
        .prepared_action()
        .expect("subject preserves prepared_action when request carries it");
    assert_eq!(prepared.action(), &pa);
}

#[test]
fn subject_builder_preserves_none_when_prepared_action_absent() {
    let captured = Arc::new(Mutex::new(None));
    let use_case = capturing_use_case(inspect_action(), Arc::clone(&captured));

    let request = request_with_prepared_action(
        OperatorMode::Read,
        AllowedTools::for_mode(OperatorMode::Read),
        None,
    );
    let _outcome = use_case.execute(&request).expect("predict succeeds");

    let subject = captured.lock().unwrap();
    let subject = subject.as_ref().expect("policy received subject");
    assert!(
        subject.prepared_action().is_none(),
        "subject preserves None when request has no prepared_action"
    );
}

#[test]
fn prepared_action_with_terminal_kind_is_rejected_as_subject_build_failure() {
    // PreparedOperatorAction requires a tool_call; a Stop action must not
    // sneak through as prepared_action. The runtime should surface this as a
    // SubjectBuild failure rather than silently dropping it.
    let stop = stop_action();
    let captured = Arc::new(Mutex::new(None));
    let use_case = capturing_use_case(inspect_action(), Arc::clone(&captured));

    let request = request_with_prepared_action(
        OperatorMode::Read,
        AllowedTools::for_mode(OperatorMode::Read),
        Some(stop),
    );
    let err = use_case
        .execute(&request)
        .expect_err("stop as prepared_action must error during subject build");
    assert!(matches!(err, RuntimeError::SubjectBuild { .. }));
}
