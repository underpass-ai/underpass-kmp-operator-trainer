//! End-to-end round-trip tests across the full mapper stack and the
//! JSONL adapters. These tests live in `infra` (not `domain`) because
//! they exercise the wire format directly.

use std::io::Write;

use operator_shared_application::ports::trajectory_reader::TrajectoryReader;
use operator_shared_application::ports::trajectory_writer::TrajectoryWriter;
use operator_shared_domain::action::operator_action::OperatorAction;
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
use operator_shared_domain::value_objects::task_family::TaskFamily;
use operator_shared_domain::visible_state::visible_state_builder::VisibleStateBuilder;

use crate::adapters::jsonl::jsonl_trajectory_reader::JsonlTrajectoryReader;
use crate::adapters::jsonl::jsonl_trajectory_writer::JsonlTrajectoryWriter;

fn build_inspect_trajectory() -> TrainingTrajectory {
    let target = MemoryRef::parse("node:42").unwrap();
    let visible = VisibleStateBuilder::new()
        .with_known_ref(target.clone())
        .build();
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
        InspectArguments::new(target),
    )));
    TrainingTrajectory::new(
        TrainingTrajectoryId::parse("traj:42").unwrap(),
        StepId::parse("step:42").unwrap(),
        AboutId::parse("about:42").unwrap(),
        OperatorMode::Read,
        TaskFamily::parse("read.inspect").unwrap(),
        AllowedTools::for_mode(OperatorMode::Read),
        visible,
        action,
    )
    .unwrap()
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
    let original = build_inspect_trajectory();
    writer.write(&original).expect("write succeeds");
    drop(writer);

    let reader = JsonlTrajectoryReader::new(&path);
    let read_back = reader.read_all().expect("read succeeds");
    assert_eq!(read_back.len(), 1);
    assert_eq!(&read_back[0], &original);

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
    let _ = err; // shape is internal to the application error
    std::fs::remove_file(&path).expect("cleanup");
}
