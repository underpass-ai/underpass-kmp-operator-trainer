//! End-to-end round-trip tests across the full mapper stack and the
//! JSONL adapters. These tests live in `infra` (not `domain`) because
//! they exercise the wire format directly.

use std::io::Write;

use operator_shared_application::ports::trajectory_reader::TrajectoryReader;
use operator_shared_application::ports::trajectory_writer::TrajectoryWriter;
use operator_shared_domain::action::escalate_action::EscalateAction;
use operator_shared_domain::action::escalate_reason::EscalateReason;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::action::stop_action::StopAction;
use operator_shared_domain::action::stop_reason::StopReason;
use operator_shared_domain::action::tool_call_action::ToolCallAction;
use operator_shared_domain::cursor::around_cursor::AroundCursor;
use operator_shared_domain::cursor::cursor::Cursor;
use operator_shared_domain::cursor::ref_cursor::RefCursor;
use operator_shared_domain::cursor::temporal_anchor::TemporalAnchor;
use operator_shared_domain::cursor::temporal_cursor::TemporalCursor;
use operator_shared_domain::cursor::temporal_cursor_key::TemporalCursorKey;
use operator_shared_domain::cursor::trace_cursor::TraceCursor;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::ids::step_id::StepId;
use operator_shared_domain::ids::training_trajectory_id::TrainingTrajectoryId;
use operator_shared_domain::mode::allowed_tools::AllowedTools;
use operator_shared_domain::mode::operator_mode::OperatorMode;
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
use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;
use operator_shared_domain::value_objects::dimension_ref::DimensionRef;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::model_id::ModelId;
use operator_shared_domain::value_objects::positive_count::PositiveCount;
use operator_shared_domain::value_objects::task_family::TaskFamily;
use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
use operator_shared_domain::visible_state::visible_state::VisibleState;

use crate::adapters::jsonl::jsonl_trajectory_reader::JsonlTrajectoryReader;
use crate::adapters::jsonl::jsonl_trajectory_writer::JsonlTrajectoryWriter;
use crate::mappers::training_trajectory_mapper::TrainingTrajectoryMapper;

fn read_mode_trajectory(action: OperatorAction, family: &str) -> TrainingTrajectory {
    let target = MemoryRef::parse("node:42").unwrap();
    let dim = DimensionRef::parse("temporal").unwrap();
    let visible = VisibleState::assemble(
        [target, MemoryRef::parse("node:other").unwrap()],
        [dim],
        Some(Cursor::Temporal(TemporalCursor::new(
            TemporalCursorKey::Created,
            TemporalAnchor::parse("2026-05-18T00:00:00Z").unwrap(),
        ))),
        BudgetSnapshot::bounded(10, 1000),
    );
    TrainingTrajectory::new(
        TrainingTrajectoryId::parse("traj:1").unwrap(),
        StepId::parse("step:1").unwrap(),
        AboutId::parse("about:1").unwrap(),
        OperatorMode::Read,
        TaskFamily::parse(family).unwrap(),
        TrajectoryGoal::parse(format!("Execute the {family} operator step.")).unwrap(),
        AllowedTools::for_mode(OperatorMode::Read),
        visible,
        action,
    )
    .unwrap()
}

fn round_trip(trajectory: &TrainingTrajectory) {
    let dto = TrainingTrajectoryMapper::to_dto(trajectory).unwrap();
    let back = TrainingTrajectoryMapper::to_domain(&dto).unwrap();
    assert_eq!(trajectory, &back);
}

#[test]
fn wake_round_trips() {
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Wake(
        WakeArguments::new(AboutId::parse("about:topic").unwrap()),
    )));
    round_trip(&read_mode_trajectory(action, "read.wake"));
}

#[test]
fn ask_round_trips() {
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Ask(
        AskArguments::new("why did X happen").unwrap(),
    )));
    round_trip(&read_mode_trajectory(action, "read.ask"));
}

#[test]
fn near_round_trips() {
    let anchor = MemoryRef::parse("node:42").unwrap();
    let dim = DimensionRef::parse("temporal").unwrap();
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Near(
        NearArguments::new(
            anchor,
            vec![dim],
            Some(PositiveCount::parse(5, "limit").unwrap()),
        )
        .unwrap(),
    )));
    round_trip(&read_mode_trajectory(action, "read.near"));
}

#[test]
fn near_with_window_round_trips() {
    let anchor = MemoryRef::parse("node:42").unwrap();
    let dim = DimensionRef::parse("temporal").unwrap();
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Near(
        NearArguments::new(anchor, vec![dim], None)
            .unwrap()
            .with_window(4, 6),
    )));
    round_trip(&read_mode_trajectory(action, "read.near.window"));
}

#[test]
fn goto_ref_round_trips() {
    let target = MemoryRef::parse("node:42").unwrap();
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Goto(
        GotoArguments::new(Cursor::Ref(RefCursor::new(target))),
    )));
    round_trip(&read_mode_trajectory(action, "read.goto.ref"));
}

#[test]
fn goto_around_round_trips() {
    let anchor = MemoryRef::parse("node:42").unwrap();
    let dim = DimensionRef::parse("temporal").unwrap();
    let around = AroundCursor::new(anchor, vec![dim]).unwrap();
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Goto(
        GotoArguments::new(Cursor::Around(around)),
    )));
    round_trip(&read_mode_trajectory(action, "read.goto.around"));
}

#[test]
fn goto_trace_round_trips() {
    let from = MemoryRef::parse("node:42").unwrap();
    let to = MemoryRef::parse("node:other").unwrap();
    let trace = TraceCursor::new(from, to);
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Goto(
        GotoArguments::new(Cursor::Trace(trace)),
    )));
    round_trip(&read_mode_trajectory(action, "read.goto.trace"));
}

#[test]
fn rewind_round_trips() {
    let cursor = TemporalCursor::new(
        TemporalCursorKey::Created,
        TemporalAnchor::parse("2026-05-18T00:00:00Z").unwrap(),
    );
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Rewind(
        RewindArguments::new(cursor, PositiveCount::parse(3, "window").unwrap()),
    )));
    round_trip(&read_mode_trajectory(action, "read.rewind"));
}

#[test]
fn forward_round_trips() {
    let cursor = TemporalCursor::new(
        TemporalCursorKey::Created,
        TemporalAnchor::parse("2026-05-18T00:00:00Z").unwrap(),
    );
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Forward(
        ForwardArguments::new(cursor, PositiveCount::parse(3, "window").unwrap()),
    )));
    round_trip(&read_mode_trajectory(action, "read.forward"));
}

#[test]
fn trace_round_trips() {
    let from = MemoryRef::parse("node:42").unwrap();
    let to = MemoryRef::parse("node:other").unwrap();
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Trace(
        TraceArguments::new(from, Some(to), PositiveCount::parse(1, "page").unwrap()),
    )));
    round_trip(&read_mode_trajectory(action, "read.trace"));
}

#[test]
fn inspect_round_trips() {
    let target = MemoryRef::parse("node:42").unwrap();
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
        InspectArguments::new(target),
    )));
    round_trip(&read_mode_trajectory(action, "read.inspect"));
}

#[test]
fn write_memory_round_trips() {
    let related = vec![MemoryRef::parse("node:1").unwrap()];
    let visible = VisibleState::assemble([], [], None, BudgetSnapshot::unbounded());
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::WriteMemory(
        WriteMemoryArguments::new("summary text", "body text", related).unwrap(),
    )));
    let trajectory = TrainingTrajectory::new(
        TrainingTrajectoryId::parse("traj:wm").unwrap(),
        StepId::parse("step:wm").unwrap(),
        AboutId::parse("about:wm").unwrap(),
        OperatorMode::Write,
        TaskFamily::parse("write.memory").unwrap(),
        TrajectoryGoal::parse("Write the prepared memory note.").unwrap(),
        AllowedTools::for_mode(OperatorMode::Write),
        visible,
        action,
    )
    .unwrap();
    round_trip(&trajectory);
}

#[test]
fn stop_action_round_trips() {
    let evidence = vec![MemoryRef::parse("node:42").unwrap()];
    let action = OperatorAction::Stop(
        StopAction::new(
            StopReason::AnswerReady,
            Some("the answer".to_string()),
            evidence,
        )
        .unwrap(),
    );
    round_trip(&read_mode_trajectory(action, "read.stop"));
}

#[test]
fn stop_action_without_answer_round_trips() {
    let action =
        OperatorAction::Stop(StopAction::new(StopReason::BudgetExhausted, None, vec![]).unwrap());
    round_trip(&read_mode_trajectory(action, "read.stop.empty"));
}

#[test]
fn escalate_action_round_trips() {
    let action = OperatorAction::Escalate(EscalateAction::new(
        EscalateReason::BeyondCapability,
        ModelId::parse("claude-opus-4-7").unwrap(),
    ));
    round_trip(&read_mode_trajectory(action, "read.escalate"));
}

#[test]
fn unbounded_budget_round_trips() {
    let visible = VisibleState::assemble(
        [MemoryRef::parse("node:42").unwrap()],
        [],
        None,
        BudgetSnapshot::unbounded(),
    );
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
        InspectArguments::new(MemoryRef::parse("node:42").unwrap()),
    )));
    let trajectory = TrainingTrajectory::new(
        TrainingTrajectoryId::parse("traj:un").unwrap(),
        StepId::parse("step:un").unwrap(),
        AboutId::parse("about:un").unwrap(),
        OperatorMode::Read,
        TaskFamily::parse("read.inspect").unwrap(),
        TrajectoryGoal::parse("Inspect the visible memory node with an unbounded budget.").unwrap(),
        AllowedTools::for_mode(OperatorMode::Read),
        visible,
        action,
    )
    .unwrap();
    round_trip(&trajectory);
}

#[test]
fn jsonl_writer_then_reader_round_trips_a_trajectory() {
    let tmp_dir = std::env::temp_dir();
    let pid = std::process::id();
    let path = tmp_dir.join(format!("operator-shared-infra-round-trip-{pid}.jsonl"));
    if path.exists() {
        std::fs::remove_file(&path).expect("clean previous fixture");
    }

    let mut writer = JsonlTrajectoryWriter::new(&path);
    let target = MemoryRef::parse("node:42").unwrap();
    let visible = VisibleState::assemble([target.clone()], [], None, BudgetSnapshot::unbounded());
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
        InspectArguments::new(target),
    )));
    let original = TrainingTrajectory::new(
        TrainingTrajectoryId::parse("traj:42").unwrap(),
        StepId::parse("step:42").unwrap(),
        AboutId::parse("about:42").unwrap(),
        OperatorMode::Read,
        TaskFamily::parse("read.inspect").unwrap(),
        TrajectoryGoal::parse("Inspect the visible memory node.").unwrap(),
        AllowedTools::for_mode(OperatorMode::Read),
        visible,
        action,
    )
    .unwrap();
    writer.write(&original).expect("write succeeds");
    writer.write(&original).expect("second write succeeds");
    drop(writer);

    let reader = JsonlTrajectoryReader::new(&path);
    let read_back = reader.read_all().expect("read succeeds");
    assert_eq!(read_back.len(), 2);
    assert_eq!(&read_back[0], &original);
    assert_eq!(&read_back[1], &original);

    std::fs::remove_file(&path).expect("cleanup");
}

#[test]
fn reader_rejects_malformed_lines() {
    let tmp_dir = std::env::temp_dir();
    let pid = std::process::id();
    let path = tmp_dir.join(format!("operator-shared-infra-malformed-{pid}.jsonl"));
    if path.exists() {
        std::fs::remove_file(&path).expect("clean previous fixture");
    }
    {
        let mut file = std::fs::File::create(&path).expect("create fixture");
        file.write_all(b"{not json}\n").expect("write fixture");
    }
    let reader = JsonlTrajectoryReader::new(&path);
    let err = reader.read_all().expect_err("malformed line must fail");
    let _ = err;
    std::fs::remove_file(&path).expect("cleanup");
}

#[test]
fn reader_skips_blank_lines() {
    let tmp_dir = std::env::temp_dir();
    let pid = std::process::id();
    let path = tmp_dir.join(format!("operator-shared-infra-blank-{pid}.jsonl"));
    if path.exists() {
        std::fs::remove_file(&path).expect("clean previous fixture");
    }
    let mut writer = JsonlTrajectoryWriter::new(&path);
    let target = MemoryRef::parse("node:42").unwrap();
    let visible = VisibleState::assemble([target.clone()], [], None, BudgetSnapshot::unbounded());
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
        InspectArguments::new(target),
    )));
    let original = TrainingTrajectory::new(
        TrainingTrajectoryId::parse("traj:blank").unwrap(),
        StepId::parse("step:1").unwrap(),
        AboutId::parse("about:1").unwrap(),
        OperatorMode::Read,
        TaskFamily::parse("read.inspect").unwrap(),
        TrajectoryGoal::parse("Inspect the visible memory node.").unwrap(),
        AllowedTools::for_mode(OperatorMode::Read),
        visible,
        action,
    )
    .unwrap();
    writer.write(&original).expect("write");
    drop(writer);
    // append a blank line
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .expect("reopen");
    file.write_all(b"\n   \n").expect("blank lines");
    drop(file);

    let reader = JsonlTrajectoryReader::new(&path);
    let read_back = reader.read_all().expect("read succeeds");
    assert_eq!(read_back.len(), 1);

    std::fs::remove_file(&path).expect("cleanup");
}
