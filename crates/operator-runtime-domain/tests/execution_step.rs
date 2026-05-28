use std::time::Duration;

use operator_runtime_domain::budget::session_budget::SessionBudget;
use operator_runtime_domain::session::execution_step::ExecutionStep;
use operator_runtime_domain::session::observation::Observation;
use operator_runtime_domain::session::observation_error_code::ObservationErrorCode;
use operator_runtime_domain::session::operator_request::OperatorRequest;
use operator_runtime_domain::session::operator_session_id::OperatorSessionId;
use operator_runtime_domain::session::outcome_class::OutcomeClass;
use operator_runtime_domain::session::session_outcome::SessionOutcome;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::action::tool_call_action::ToolCallAction;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::mode::allowed_tools::AllowedTools;
use operator_shared_domain::mode::operator_mode::OperatorMode;
use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
use operator_shared_domain::tool_outcomes::inspect_outcome::InspectOutcome;
use operator_shared_domain::tool_outcomes::tool_outcome::ToolOutcome;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;
use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
use operator_shared_domain::visible_state::visible_state::VisibleState;

fn text(value: &str) -> NonEmptyString {
    NonEmptyString::parse(value, "runtime_domain_test").unwrap()
}

fn target() -> MemoryRef {
    MemoryRef::parse("node:1").unwrap()
}

fn inspect_action() -> OperatorAction {
    OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
        InspectArguments::new(target()),
    )))
}

fn inspect_observation() -> Observation {
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

fn request() -> OperatorRequest {
    OperatorRequest::new(
        OperatorSessionId::parse("session:step").unwrap(),
        TrajectoryGoal::parse("Inspect node one.").unwrap(),
        VisibleState::assemble([target()], [], None, BudgetSnapshot::bounded(1, 4096)),
        OperatorMode::Read,
        AllowedTools::for_mode(OperatorMode::Read),
        SessionBudget::new(1, 4096),
        AboutId::parse("about:test").unwrap(),
        None,
    )
    .unwrap()
}

#[test]
fn execution_step_carries_action_and_observation() {
    let action = inspect_action();
    let observation = inspect_observation();

    let step = ExecutionStep::new(action.clone(), observation.clone());

    assert_eq!(step.action(), &action);
    assert_eq!(step.observation(), &observation);
}

#[test]
fn session_outcome_from_tool_response_consumes_one_call() {
    let action = inspect_action();
    let outcome = SessionOutcome::from_observation(
        &request(),
        action,
        inspect_observation(),
        Duration::from_millis(12),
    )
    .expect("budget allows a call");

    assert!(matches!(outcome.outcome_class(), OutcomeClass::Completed));
    assert_eq!(outcome.final_budget().calls_remaining(), 0);
    assert_eq!(outcome.elapsed_ms(), 12);
}

#[test]
fn session_outcome_from_tool_error_preserves_error_code() {
    let code = ObservationErrorCode::parse("kernel_rejected").unwrap();
    let outcome = SessionOutcome::from_observation(
        &request(),
        inspect_action(),
        Observation::ToolError {
            code: code.clone(),
            message: "kernel rejected request".to_string(),
        },
        Duration::from_millis(1),
    )
    .expect("budget allows a call");

    assert_eq!(
        outcome.outcome_class(),
        &OutcomeClass::McpExecutionFailure { code }
    );
}
