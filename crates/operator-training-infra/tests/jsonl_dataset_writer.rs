//! Filesystem round-trip tests for `JsonlSftDatasetWriter`. Writes
//! one trajectory through the real adapter, reads the JSONL back,
//! and verifies the outcome value object is self-consistent.

use std::fs;
use std::io::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};

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
use operator_shared_domain::value_objects::task_family::TaskFamily;
use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
use operator_shared_domain::visible_state::visible_state::VisibleState;
use operator_training_application::ports::dataset_writer::DatasetWriter;
use operator_training_infra::adapters::jsonl_sft_dataset_writer::JsonlSftDatasetWriter;
use operator_training_infra::adapters::jsonl_trajectory_dataset_writer::JsonlTrajectoryDatasetWriter;

static SEQ: AtomicU64 = AtomicU64::new(1);

fn tmp_path(label: &str) -> std::path::PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("operator-training-infra-{label}-{pid}-{n}"))
}

fn inspect_trajectory(traj: &str, family: &str, target_ref: &str) -> TrainingTrajectory {
    let target = MemoryRef::parse(target_ref).unwrap();
    let visible = VisibleState::assemble(
        [target.clone()],
        std::iter::empty(),
        None,
        BudgetSnapshot::unbounded(),
    );
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
        InspectArguments::new(target),
    )));
    TrainingTrajectory::new(
        TrainingTrajectoryId::parse(traj).unwrap(),
        StepId::parse("step:1").unwrap(),
        AboutId::parse("about:1").unwrap(),
        OperatorMode::Read,
        TaskFamily::parse(family).unwrap(),
        operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal::parse(format!(
            "Execute the {family} dataset writer fixture."
        ))
        .unwrap(),
        AllowedTools::for_mode(OperatorMode::Read),
        visible,
        action,
    )
    .unwrap()
}

fn stop_trajectory(traj: &str, answer: &str, evidence_ref: &str) -> TrainingTrajectory {
    let evidence = MemoryRef::parse(evidence_ref).unwrap();
    let visible = VisibleState::assemble(
        [evidence.clone()],
        std::iter::empty(),
        None,
        BudgetSnapshot::unbounded(),
    );
    let action = OperatorAction::Stop(
        StopAction::new(
            StopReason::AnswerReady,
            Some(answer.to_string()),
            vec![evidence],
        )
        .unwrap(),
    );
    TrainingTrajectory::new(
        TrainingTrajectoryId::parse(traj).unwrap(),
        StepId::parse("step:stop").unwrap(),
        AboutId::parse("about:1").unwrap(),
        OperatorMode::Read,
        TaskFamily::parse("runtime.cross_about_count").unwrap(),
        operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal::parse(
            "Count workshops across the venues in the period.",
        )
        .unwrap(),
        AllowedTools::for_mode(OperatorMode::Read),
        visible,
        action,
    )
    .unwrap()
}

#[test]
fn training_user_context_does_not_leak_the_target_answer() {
    // The answer (the count derivation) is the training TARGET — it must live
    // only in target_action (the assistant message), never in the user-facing
    // context the model conditions on. This guards against context leakage that
    // would let the student copy the answer instead of learning the policy.
    const SENTINEL: &str = "SENTINEL-LEAK-CHECK-42-WORKSHOPS";
    let path = tmp_path("no-leak");
    JsonlTrajectoryDatasetWriter::new(&path)
        .write(&[stop_trajectory("traj:stop", SENTINEL, "node:1")])
        .expect("write");
    let body = fs::read_to_string(&path).expect("readable");
    let row: serde_json::Value = serde_json::from_str(body.lines().next().unwrap()).unwrap();

    // The answer is present where it belongs: the assistant-bound target action.
    assert_eq!(row["target_action"]["answer"], SENTINEL);

    // Reconstruct exactly the user-facing payload the SFT-prep pipeline builds
    // (task_family/mode/about/goal/allowed_tools/visible_state — never
    // target_action) and assert the answer does not leak into it.
    let user_payload = serde_json::json!({
        "task_family": row["task_family"],
        "mode": row["mode"],
        "about": row["about"],
        "goal": row["goal"],
        "allowed_tools": row["allowed_tools"],
        "visible_state": row["visible_state"],
    });
    let user_str = serde_json::to_string(&user_payload).unwrap();
    assert!(
        !user_str.contains(SENTINEL),
        "target answer leaked into the model-facing user context: {user_str}"
    );
    // No stray action/answer fields smuggled inside the visible_state.
    assert!(row["visible_state"].get("target_action").is_none());
    assert!(row["visible_state"].get("answer").is_none());
    assert!(row["visible_state"].get("requested_stop").is_none());
}

#[test]
fn trajectory_writer_emits_rich_schema_with_about_goal_and_target_action() {
    // The rich trajectory writer carries the fields the Python SFT-prep pipeline
    // needs (about/goal/mode/task_family/allowed_tools/target_action), unlike the
    // thin {prompt,completion} SFT writer.
    let path = tmp_path("trajectory");
    let writer = JsonlTrajectoryDatasetWriter::new(&path);
    let trajectories = vec![
        inspect_trajectory("traj:1", "runtime.cross_about_count", "node:1"),
        inspect_trajectory("traj:2", "runtime.cross_about_count", "node:2"),
    ];

    let outcome = writer.write(&trajectories).expect("write");

    let body = fs::read_to_string(&path).expect("dataset readable");
    let lines: Vec<&str> = body.lines().collect();
    assert_eq!(lines.len(), 2);
    for line in &lines {
        let row: serde_json::Value = serde_json::from_str(line).expect("valid json line");
        assert_eq!(row["about"], "about:1");
        assert!(row["goal"].as_str().unwrap().contains("dataset writer fixture"));
        assert_eq!(row["mode"], "read");
        assert_eq!(row["task_family"], "runtime.cross_about_count");
        assert!(row["allowed_tools"].is_array());
        assert!(row["visible_state"].is_object());
        assert_eq!(row["target_action"]["tool"], "kernel_inspect");
        // None of the thin-writer keys leak in.
        assert!(row.get("prompt").is_none());
        assert!(row.get("completion").is_none());
    }
    assert!(outcome.content_hash().as_str().starts_with("sha256:"));
    assert_eq!(outcome.trajectory_count().as_usize(), 2);
}

#[test]
fn writes_one_jsonl_line_per_trajectory_and_hashes_bytes() {
    let path = tmp_path("happy");
    let writer = JsonlSftDatasetWriter::new(&path);
    let trajectories = vec![
        inspect_trajectory("traj:1", "read.inspect", "node:1"),
        inspect_trajectory("traj:2", "read.inspect", "node:2"),
        inspect_trajectory("traj:3", "read.ask", "node:3"),
    ];

    let outcome = writer.write(&trajectories).expect("write");

    let body = fs::read_to_string(&path).expect("dataset readable");
    let lines: Vec<&str> = body.lines().collect();
    assert_eq!(lines.len(), 3, "one line per trajectory");
    for line in &lines {
        assert!(line.contains(r#""prompt":"#));
        assert!(line.contains(r#""completion":"#));
        assert!(line.contains("kernel_inspect"));
    }

    assert!(outcome.content_hash().as_str().starts_with("sha256:"));
    assert_eq!(outcome.trajectory_count().as_usize(), 3);

    let distribution = outcome.distribution();
    assert_eq!(distribution.family_count(), 2);
    let entries = distribution.entries();
    let by_family: std::collections::BTreeMap<_, _> = entries
        .iter()
        .map(|e| (e.family().as_str().to_string(), e.count().as_usize()))
        .collect();
    assert_eq!(by_family.get("read.inspect"), Some(&2));
    assert_eq!(by_family.get("read.ask"), Some(&1));

    fs::remove_file(&path).ok();
}

#[test]
fn content_hash_is_deterministic_across_runs_with_same_input() {
    let trajectories = vec![inspect_trajectory("traj:1", "read.inspect", "node:1")];

    let path_a = tmp_path("det-a");
    let writer_a = JsonlSftDatasetWriter::new(&path_a);
    let out_a = writer_a.write(&trajectories).unwrap();

    let path_b = tmp_path("det-b");
    let writer_b = JsonlSftDatasetWriter::new(&path_b);
    let out_b = writer_b.write(&trajectories).unwrap();

    assert_eq!(out_a.content_hash().as_str(), out_b.content_hash().as_str());

    let body_a = fs::read_to_string(&path_a).unwrap();
    let body_b = fs::read_to_string(&path_b).unwrap();
    assert_eq!(body_a, body_b);

    fs::remove_file(&path_a).ok();
    fs::remove_file(&path_b).ok();
}

#[test]
fn write_to_unwritable_path_surfaces_write_failure() {
    let writer = JsonlSftDatasetWriter::new("/this/path/does/not/exist/dataset.jsonl");
    let err = writer
        .write(&[inspect_trajectory("traj:1", "read.inspect", "node:1")])
        .expect_err("must fail");
    match err {
        operator_training_application::errors::dataset_write_error::DatasetWriteError::WriteFailure {
            adapter,
            message,
        } => {
            assert_eq!(adapter, "jsonl_sft_dataset_writer");
            assert!(message.contains("create") || message.contains("No such file"));
        }
        operator_training_application::errors::dataset_write_error::DatasetWriteError::DerivedValueFailure { .. } => {
            panic!("expected WriteFailure, got DerivedValueFailure");
        }
    }
}

#[test]
fn empty_trajectory_list_yields_derived_value_failure() {
    let path = tmp_path("empty");
    let writer = JsonlSftDatasetWriter::new(&path);
    let err = writer.write(&[]).expect_err("must fail on empty");
    match err {
        operator_training_application::errors::dataset_write_error::DatasetWriteError::DerivedValueFailure {
            adapter,
            message,
        } => {
            assert_eq!(adapter, "jsonl_sft_dataset_writer");
            assert!(message.contains("trajectory_count"));
        }
        operator_training_application::errors::dataset_write_error::DatasetWriteError::WriteFailure { .. } => {
            panic!("expected DerivedValueFailure, got WriteFailure");
        }
    }
    // file may exist as zero-length; clean up.
    fs::remove_file(&path).ok();
}

#[test]
fn appends_a_trailing_newline_after_the_final_line() {
    let path = tmp_path("trailing");
    let writer = JsonlSftDatasetWriter::new(&path);
    let trajectories = vec![inspect_trajectory("traj:1", "read.inspect", "node:1")];
    writer.write(&trajectories).unwrap();

    let body = fs::read_to_string(&path).unwrap();
    assert!(body.ends_with('\n'), "JSONL must end with a newline");

    fs::remove_file(&path).ok();
}

// Smoke test that proves the writer is `Send + Sync` — the port
// requires both, and the test fails at compile time if it stops
// holding.
#[test]
fn writer_is_send_and_sync() {
    fn require_send_sync<T: Send + Sync>(_: &T) {}
    let writer = JsonlSftDatasetWriter::new("/tmp/whatever");
    require_send_sync(&writer);
}

#[test]
fn dropping_a_partial_write_does_not_leave_a_stale_handle() {
    // Sanity: writing twice to the same path overwrites cleanly.
    let path = tmp_path("overwrite");
    let writer = JsonlSftDatasetWriter::new(&path);

    let first = vec![inspect_trajectory("traj:1", "read.inspect", "node:1")];
    let second = vec![
        inspect_trajectory("traj:1", "read.inspect", "node:1"),
        inspect_trajectory("traj:2", "read.ask", "node:2"),
    ];

    writer.write(&first).unwrap();
    let body1 = fs::read_to_string(&path).unwrap();
    assert_eq!(body1.lines().count(), 1);

    writer.write(&second).unwrap();
    let body2 = fs::read_to_string(&path).unwrap();
    assert_eq!(body2.lines().count(), 2);

    fs::remove_file(&path).ok();
}

// Helper to verify that a non-empty path with garbage in it gets
// overwritten when the writer runs (we never want to be in append
// mode by accident).
#[test]
fn writer_overwrites_existing_target_file() {
    let path = tmp_path("preexisting");
    {
        let mut handle = fs::File::create(&path).unwrap();
        handle.write_all(b"garbage\n").unwrap();
    }

    let writer = JsonlSftDatasetWriter::new(&path);
    let trajectories = vec![inspect_trajectory("traj:1", "read.inspect", "node:1")];
    writer.write(&trajectories).unwrap();

    let body = fs::read_to_string(&path).unwrap();
    assert!(!body.contains("garbage"));
    assert_eq!(body.lines().count(), 1);

    fs::remove_file(&path).ok();
}
