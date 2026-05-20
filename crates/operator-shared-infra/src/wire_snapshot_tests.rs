//! Snapshot tests that fix the canonical JSON wire shape of one
//! trajectory per variant. They go through `serde_json::to_string` (not
//! only through `TrainingTrajectoryMapper::to_dto`) to catch breaking
//! serde changes — for example a different field order from `#[serde(flatten)]`
//! interactions or a renamed tag.

use operator_shared_contract::operator_action_dto::OperatorActionDto;
use operator_shared_contract::training_trajectory_dto::TrainingTrajectoryDto;
use operator_shared_domain::action::escalate_action::EscalateAction;
use operator_shared_domain::action::escalate_reason::EscalateReason;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::action::stop_action::StopAction;
use operator_shared_domain::action::stop_reason::StopReason;
use operator_shared_domain::action::tool_call_action::ToolCallAction;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::ids::step_id::StepId;
use operator_shared_domain::ids::training_trajectory_id::TrainingTrajectoryId;
use operator_shared_domain::mode::allowed_tools::AllowedTools;
use operator_shared_domain::mode::operator_mode::OperatorMode;
use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::model_id::ModelId;
use operator_shared_domain::value_objects::task_family::TaskFamily;
use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
use operator_shared_domain::visible_state::visible_state::VisibleState;

use crate::mappers::operator_action_mapper::OperatorActionMapper;
use crate::mappers::training_trajectory_mapper::TrainingTrajectoryMapper;

fn inspect_trajectory() -> TrainingTrajectory {
    let target = MemoryRef::parse("node:1").unwrap();
    let visible =
        VisibleState::assemble([target.clone()], [], None, BudgetSnapshot::bounded(5, 1000));
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
        InspectArguments::new(target),
    )));
    TrainingTrajectory::new(
        TrainingTrajectoryId::parse("traj:snap").unwrap(),
        StepId::parse("step:snap").unwrap(),
        AboutId::parse("about:snap").unwrap(),
        OperatorMode::Read,
        TaskFamily::parse("read.inspect").unwrap(),
        TrajectoryGoal::parse("Inspect node:1.").unwrap(),
        AllowedTools::for_mode(OperatorMode::Read),
        visible,
        action,
    )
    .unwrap()
}

#[test]
fn tool_call_action_serializes_to_canonical_json() {
    let dto = OperatorActionMapper::to_dto(&OperatorAction::ToolCall(ToolCallAction::new(
        ToolArguments::Inspect(InspectArguments::new(MemoryRef::parse("node:1").unwrap())),
    )))
    .unwrap();
    let json = serde_json::to_string(&dto).unwrap();
    assert_eq!(
        json,
        r#"{"kind":"tool_call","tool":"kernel_inspect","arguments":{"target":"node:1"}}"#
    );
    let parsed: OperatorActionDto = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, dto);
}

#[test]
fn stop_action_serializes_to_canonical_json() {
    let dto = OperatorActionMapper::to_dto(&OperatorAction::Stop(
        StopAction::new(
            StopReason::AnswerReady,
            Some("the answer".to_string()),
            vec![MemoryRef::parse("node:1").unwrap()],
        )
        .unwrap(),
    ))
    .unwrap();
    let json = serde_json::to_string(&dto).unwrap();
    assert_eq!(
        json,
        r#"{"kind":"stop","reason":"answer_ready","answer":"the answer","evidence":["node:1"]}"#
    );
    let parsed: OperatorActionDto = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, dto);
}

#[test]
fn stop_action_without_optionals_omits_fields() {
    let dto = OperatorActionMapper::to_dto(&OperatorAction::Stop(
        StopAction::new(StopReason::NoCandidate, None, vec![]).unwrap(),
    ))
    .unwrap();
    let json = serde_json::to_string(&dto).unwrap();
    assert_eq!(json, r#"{"kind":"stop","reason":"no_candidate"}"#);
    let parsed: OperatorActionDto = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, dto);
}

#[test]
fn escalate_action_serializes_to_canonical_json() {
    let dto = OperatorActionMapper::to_dto(&OperatorAction::Escalate(EscalateAction::new(
        EscalateReason::LowConfidence,
        ModelId::parse("claude-opus-4-7").unwrap(),
    )))
    .unwrap();
    let json = serde_json::to_string(&dto).unwrap();
    assert_eq!(
        json,
        r#"{"kind":"escalate","reason":"low_confidence","target_model":"claude-opus-4-7"}"#
    );
    let parsed: OperatorActionDto = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, dto);
}

#[test]
fn full_inspect_trajectory_round_trips_through_real_json() {
    let trajectory = inspect_trajectory();
    let dto = TrainingTrajectoryMapper::to_dto(&trajectory).unwrap();
    let json = serde_json::to_string(&dto).unwrap();
    let parsed: TrainingTrajectoryDto = serde_json::from_str(&json).unwrap();
    let back = TrainingTrajectoryMapper::to_domain(&parsed).unwrap();
    assert_eq!(trajectory, back);
}

#[test]
fn full_trajectory_without_goal_is_invalid_json() {
    let json = concat!(
        r#"{"id":"traj:snap","step_id":"step:snap","about":"about:snap","mode":"read","#,
        r#""task_family":"read.inspect","#,
        r#""allowed_tools":["kernel_wake","kernel_ask","kernel_near","kernel_goto",
"kernel_rewind","kernel_forward","kernel_trace","kernel_inspect"],"#,
        r#""visible_state":{"known_refs":["node:1"],"known_dimensions":[],"budget":{"calls_remaining":5,"tokens_remaining":1000}},"#,
        r#""target_action":{"kind":"tool_call","tool":"kernel_inspect","arguments":{"target":"node:1"}}}"#
    )
    .replace('\n', "");
    assert!(serde_json::from_str::<TrainingTrajectoryDto>(&json).is_err());
}

#[test]
fn full_inspect_trajectory_canonical_json_shape() {
    let trajectory = inspect_trajectory();
    let dto = TrainingTrajectoryMapper::to_dto(&trajectory).unwrap();
    let json = serde_json::to_string(&dto).unwrap();
    // Field order is the order serde emits given the DTO definitions. If
    // this breaks, the wire contract changed; either restore the order or
    // accept the change and update this snapshot.
    assert_eq!(
        json,
        concat!(
            r#"{"id":"traj:snap","step_id":"step:snap","about":"about:snap","mode":"read","#,
            r#""task_family":"read.inspect","goal":"Inspect node:1.","#,
            r#""allowed_tools":["kernel_wake","kernel_ask","kernel_near","kernel_goto",
"kernel_rewind","kernel_forward","kernel_trace","kernel_inspect"],"#,
            r#""visible_state":{"known_refs":["node:1"],"known_dimensions":[],"budget":{"calls_remaining":5,"tokens_remaining":1000}},"#,
            r#""target_action":{"kind":"tool_call","tool":"kernel_inspect","arguments":{"target":"node:1"}}}"#
        )
        .replace('\n', "")
    );
}
