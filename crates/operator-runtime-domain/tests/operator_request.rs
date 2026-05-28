use operator_runtime_domain::budget::session_budget::SessionBudget;
use operator_runtime_domain::error::runtime_domain_error::RuntimeDomainError;
use operator_runtime_domain::session::operator_request::OperatorRequest;
use operator_runtime_domain::session::operator_session_id::OperatorSessionId;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::action::tool_call_action::ToolCallAction;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::mode::allowed_tools::AllowedTools;
use operator_shared_domain::mode::operator_mode::OperatorMode;
use operator_shared_domain::tool_arguments::ask_arguments::AskArguments;
use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
use operator_shared_domain::visible_state::visible_state::VisibleState;

fn visible_state() -> VisibleState {
    VisibleState::assemble([], [], None, BudgetSnapshot::bounded(1, 4096))
}

fn session_id() -> OperatorSessionId {
    OperatorSessionId::parse("session:test").unwrap()
}

fn goal() -> TrajectoryGoal {
    TrajectoryGoal::parse("Inspect visible state.").unwrap()
}

fn about() -> AboutId {
    AboutId::parse("about:test").unwrap()
}

fn ask_action(query: &str) -> OperatorAction {
    OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Ask(
        AskArguments::new(query).unwrap(),
    )))
}

#[test]
fn builds_read_request_with_read_tools() {
    let request = OperatorRequest::new(
        session_id(),
        goal(),
        visible_state(),
        OperatorMode::Read,
        AllowedTools::for_mode(OperatorMode::Read),
        SessionBudget::new(1, 4096),
        about(),
        None,
    )
    .expect("read request is valid");

    assert_eq!(request.mode(), OperatorMode::Read);
    assert_eq!(request.initial_budget().calls_remaining(), 1);
    assert!(request.prepared_action().is_none());
}

#[test]
fn rejects_allowed_tools_for_different_mode() {
    let err = OperatorRequest::new(
        session_id(),
        goal(),
        visible_state(),
        OperatorMode::Read,
        AllowedTools::for_mode(OperatorMode::Write),
        SessionBudget::new(1, 4096),
        about(),
        None,
    )
    .expect_err("write tools do not match read mode");

    assert!(matches!(
        err,
        RuntimeDomainError::AllowedToolsModeMismatch { .. }
    ));
}

#[test]
fn carries_prepared_action_when_supplied() {
    let pa = ask_action("Which constraint blocks the migration plan?");
    let request = OperatorRequest::new(
        session_id(),
        goal(),
        visible_state(),
        OperatorMode::Read,
        AllowedTools::for_mode(OperatorMode::Read),
        SessionBudget::new(1, 4096),
        about(),
        Some(pa.clone()),
    )
    .expect("read request with kernel_ask prepared_action is valid");

    assert_eq!(request.prepared_action(), Some(&pa));
}

#[test]
fn default_prepared_action_is_none() {
    let request = OperatorRequest::new(
        session_id(),
        goal(),
        visible_state(),
        OperatorMode::Read,
        AllowedTools::for_mode(OperatorMode::Read),
        SessionBudget::new(1, 4096),
        about(),
        None,
    )
    .expect("read request is valid");

    assert!(request.prepared_action().is_none());
}

#[test]
fn operator_session_id_round_trips_through_display() {
    let parsed = OperatorSessionId::parse("session:roundtrip").unwrap();
    let rendered = parsed.to_string();

    assert_eq!(
        OperatorSessionId::parse(rendered).unwrap().as_str(),
        "session:roundtrip"
    );
}
