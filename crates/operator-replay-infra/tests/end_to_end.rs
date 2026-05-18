//! End-to-end test for the replay context. Wires
//! `ExecuteReplayUseCase` with the `InMemoryKmpMcpClient` stub and
//! verifies dispatch + aggregation across every action variant.

use operator_replay_application::use_cases::execute_replay_use_case::ExecuteReplayUseCase;
use operator_replay_domain::outcome::replay_failure_reason::ReplayFailureReason;
use operator_replay_domain::outcome::replay_outcome::ReplayOutcome;
use operator_replay_domain::prediction::replay_prediction::ReplayPrediction;
use operator_replay_infra::adapters::in_memory_kmp_mcp_client::{
    FailureMode, InMemoryKmpMcpClient,
};
use operator_shared_domain::action::escalate_action::EscalateAction;
use operator_shared_domain::action::escalate_reason::EscalateReason;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::action::stop_action::StopAction;
use operator_shared_domain::action::stop_reason::StopReason;
use operator_shared_domain::action::tool_call_action::ToolCallAction;
use operator_shared_domain::cursor::cursor::Cursor;
use operator_shared_domain::cursor::ref_cursor::RefCursor;
use operator_shared_domain::cursor::temporal_anchor::TemporalAnchor;
use operator_shared_domain::cursor::temporal_cursor::TemporalCursor;
use operator_shared_domain::cursor::temporal_cursor_key::TemporalCursorKey;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::ids::training_trajectory_id::TrainingTrajectoryId;
use operator_shared_domain::tool::kernel_tool::KernelTool;
use operator_shared_domain::tool_arguments::ask_arguments::AskArguments;
use operator_shared_domain::tool_arguments::forward_arguments::ForwardArguments;
use operator_shared_domain::tool_arguments::goto_arguments::GotoArguments;
use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
use operator_shared_domain::tool_arguments::near_arguments::NearArguments;
use operator_shared_domain::tool_arguments::rewind_arguments::RewindArguments;
use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
use operator_shared_domain::tool_arguments::trace_arguments::TraceArguments;
use operator_shared_domain::tool_arguments::wake_arguments::WakeArguments;
use operator_shared_domain::tool_arguments::write_memory_arguments::WriteMemoryArguments;
use operator_shared_domain::value_objects::dimension_ref::DimensionRef;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::model_id::ModelId;
use operator_shared_domain::value_objects::positive_count::PositiveCount;

fn id(s: &str) -> TrainingTrajectoryId {
    TrainingTrajectoryId::parse(s).unwrap()
}

fn prediction(trajectory: &str, action: OperatorAction) -> ReplayPrediction {
    ReplayPrediction::new(id(trajectory), action)
}

fn tool_call(arguments: ToolArguments) -> OperatorAction {
    OperatorAction::ToolCall(ToolCallAction::new(arguments))
}

fn every_tool_predictions() -> Vec<ReplayPrediction> {
    let temporal_cursor = TemporalCursor::new(
        TemporalCursorKey::Created,
        TemporalAnchor::parse("2026-05-18T00:00:00Z").unwrap(),
    );
    vec![
        prediction(
            "t:wake",
            tool_call(ToolArguments::Wake(WakeArguments::new(
                AboutId::parse("about:1").unwrap(),
            ))),
        ),
        prediction(
            "t:ask",
            tool_call(ToolArguments::Ask(AskArguments::new("why?").unwrap())),
        ),
        prediction(
            "t:near",
            tool_call(ToolArguments::Near(
                NearArguments::new(
                    MemoryRef::parse("node:anchor").unwrap(),
                    vec![DimensionRef::parse("temporal").unwrap()],
                    None,
                )
                .unwrap(),
            )),
        ),
        prediction(
            "t:goto",
            tool_call(ToolArguments::Goto(GotoArguments::new(Cursor::Ref(
                RefCursor::new(MemoryRef::parse("node:goto").unwrap()),
            )))),
        ),
        prediction(
            "t:rewind",
            tool_call(ToolArguments::Rewind(RewindArguments::new(
                temporal_cursor.clone(),
                PositiveCount::parse(1, "window").unwrap(),
            ))),
        ),
        prediction(
            "t:forward",
            tool_call(ToolArguments::Forward(ForwardArguments::new(
                temporal_cursor,
                PositiveCount::parse(1, "window").unwrap(),
            ))),
        ),
        prediction(
            "t:trace",
            tool_call(ToolArguments::Trace(TraceArguments::new(
                MemoryRef::parse("node:from").unwrap(),
                Some(MemoryRef::parse("node:to").unwrap()),
                PositiveCount::parse(1, "page").unwrap(),
            ))),
        ),
        prediction(
            "t:inspect",
            tool_call(ToolArguments::Inspect(InspectArguments::new(
                MemoryRef::parse("node:1").unwrap(),
            ))),
        ),
        prediction(
            "t:write",
            tool_call(ToolArguments::WriteMemory(
                WriteMemoryArguments::new("s", "b", vec![]).unwrap(),
            )),
        ),
    ]
}

#[test]
fn happy_path_dispatches_every_tool_and_records_success() {
    let use_case = ExecuteReplayUseCase::new(InMemoryKmpMcpClient::ok());
    let report = use_case.execute(&every_tool_predictions()).unwrap();

    assert_eq!(report.total(), 9);
    assert_eq!(report.successful_tool_calls(), 9);
    assert_eq!(report.failed_tool_calls(), 0);
    assert_eq!(report.stop_or_escalate(), 0);
    assert!((report.tool_call_success_rate() - 1.0).abs() < f64::EPSILON);
}

#[test]
fn stop_and_escalate_are_recorded_as_no_tool_call() {
    let use_case = ExecuteReplayUseCase::new(InMemoryKmpMcpClient::ok());
    let predictions = vec![
        prediction(
            "t:stop",
            OperatorAction::Stop(StopAction::new(StopReason::AnswerReady, None, vec![]).unwrap()),
        ),
        prediction(
            "t:escalate",
            OperatorAction::Escalate(EscalateAction::new(
                EscalateReason::LowConfidence,
                ModelId::parse("claude-opus-4-7").unwrap(),
            )),
        ),
    ];

    let report = use_case.execute(&predictions).unwrap();

    assert_eq!(report.total(), 2);
    assert_eq!(report.successful_tool_calls(), 0);
    assert_eq!(report.failed_tool_calls(), 0);
    assert_eq!(report.stop_or_escalate(), 2);
    // No tool call attempted, so the rate is zero by convention.
    assert!(report.tool_call_success_rate().abs() < f64::EPSILON);
    for execution in report.executions() {
        assert!(matches!(execution.outcome(), ReplayOutcome::NoToolCall));
    }
}

#[test]
fn transport_failures_are_recorded_per_tool() {
    let use_case =
        ExecuteReplayUseCase::new(InMemoryKmpMcpClient::always_failing(FailureMode::Transport));
    let report = use_case.execute(&every_tool_predictions()).unwrap();

    assert_eq!(report.total(), 9);
    assert_eq!(report.successful_tool_calls(), 0);
    assert_eq!(report.failed_tool_calls(), 9);
    assert!(report.tool_call_success_rate().abs() < f64::EPSILON);

    // The recorded tool is the originally-predicted one.
    let inspect_failure = report
        .executions()
        .iter()
        .find(|e| e.trajectory_id().as_str() == "t:inspect")
        .expect("inspect prediction present");
    match inspect_failure.outcome() {
        ReplayOutcome::ToolCallFailed { tool, reason } => {
            assert_eq!(*tool, KernelTool::Inspect);
            assert_eq!(*reason, ReplayFailureReason::Transport);
        }
        other => panic!("expected ToolCallFailed, got {other:?}"),
    }
}

#[test]
fn mixed_run_aggregates_correctly() {
    let use_case = ExecuteReplayUseCase::new(InMemoryKmpMcpClient::ok());
    let predictions = vec![
        prediction(
            "t:inspect",
            tool_call(ToolArguments::Inspect(InspectArguments::new(
                MemoryRef::parse("node:1").unwrap(),
            ))),
        ),
        prediction(
            "t:ask",
            tool_call(ToolArguments::Ask(AskArguments::new("q").unwrap())),
        ),
        prediction(
            "t:stop",
            OperatorAction::Stop(StopAction::new(StopReason::AnswerReady, None, vec![]).unwrap()),
        ),
    ];

    let report = use_case.execute(&predictions).unwrap();

    assert_eq!(report.total(), 3);
    assert_eq!(report.successful_tool_calls(), 2);
    assert_eq!(report.stop_or_escalate(), 1);
    assert_eq!(report.failed_tool_calls(), 0);
    assert!((report.tool_call_success_rate() - 1.0).abs() < f64::EPSILON);
}
