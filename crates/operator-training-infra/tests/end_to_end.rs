//! End-to-end test: wire `BuildTrainingRunUseCase` with the real
//! filesystem adapters (`JsonlSftDatasetWriter`, `TomlManifestWriter`)
//! plus a stub `TrajectorySource`. Verifies both files land on disk
//! and the use case returns a ready `TrainingRun`.
//!
//! `ProcessTrainerInvoker` is exercised separately in
//! `process_trainer_invoker.rs`; here we focus on the dataset +
//! manifest assembly flow.

use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

use operator_evaluation_domain::report::evaluation_report::EvaluationReport;
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
use operator_shared_domain::value_objects::positive_count::PositiveCount;
use operator_shared_domain::value_objects::task_family::TaskFamily;
use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
use operator_shared_domain::visible_state::visible_state::VisibleState;
use operator_training_application::errors::trajectory_source_error::TrajectorySourceError;
use operator_training_application::ports::trajectory_source::TrajectorySource;
use operator_training_application::use_cases::build_training_run_request::BuildTrainingRunRequest;
use operator_training_application::use_cases::build_training_run_use_case::BuildTrainingRunUseCase;
use operator_training_domain::ids::TrainingRunId;
use operator_training_domain::provenance::dataset_source::DatasetSource;
use operator_training_domain::readiness::pass_rate_percent::PassRatePercent;
use operator_training_domain::readiness::readiness_gate::ReadinessGate;
use operator_training_domain::trainer::base_model_id::BaseModelId;
use operator_training_domain::trainer::output_directory::OutputDirectory;
use operator_training_domain::trainer::trainer_command::TrainerCommand;
use operator_training_domain::trainer::trainer_target::TrainerTarget;
use operator_training_infra::adapters::jsonl_sft_dataset_writer::JsonlSftDatasetWriter;
use operator_training_infra::adapters::toml_manifest_writer::TomlManifestWriter;

static SEQ: AtomicU64 = AtomicU64::new(1);

fn tmp(prefix: &str, ext: &str) -> std::path::PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("operator-training-e2e-{prefix}-{pid}-{n}.{ext}"))
}

#[derive(Debug)]
struct VecTrajectorySource {
    trajectories: Vec<TrainingTrajectory>,
}

impl TrajectorySource for VecTrajectorySource {
    fn fetch_all(&self) -> Result<Vec<TrainingTrajectory>, TrajectorySourceError> {
        Ok(self.trajectories.clone())
    }
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
        AllowedTools::for_mode(OperatorMode::Read),
        visible,
        action,
    )
    .unwrap()
}

#[test]
fn use_case_writes_dataset_and_manifest_then_returns_ready_run() {
    let dataset_path = tmp("dataset", "jsonl");
    let manifest_path = tmp("manifest", "toml");

    let source = VecTrajectorySource {
        trajectories: vec![
            inspect_trajectory("traj:1", "read.inspect", "node:1"),
            inspect_trajectory("traj:2", "read.ask", "node:2"),
        ],
    };
    let dataset_writer = JsonlSftDatasetWriter::new(&dataset_path);
    let manifest_writer = TomlManifestWriter::new(&manifest_path);
    let use_case = BuildTrainingRunUseCase::new(source, dataset_writer, manifest_writer);

    let request = BuildTrainingRunRequest::new(
        TrainingRunId::parse("run:e2e:1").unwrap(),
        TrainerTarget::new(
            TrainerCommand::parse("sft-trainer").unwrap(),
            BaseModelId::parse("base").unwrap(),
            OutputDirectory::parse("out").unwrap(),
        ),
        DatasetSource::parse("synthetic-run:test").unwrap(),
        vec![
            ReadinessGate::MinimumTrajectories(PositiveCount::parse(1, "x").unwrap()),
            ReadinessGate::MinimumTaskFamilyCoverage(PositiveCount::parse(2, "x").unwrap()),
            ReadinessGate::MinimumEvaluationPassRate(PassRatePercent::parse(0.0).unwrap()),
        ],
        EvaluationReport::from_outcomes(Vec::new()),
    );

    let run = use_case.execute(&request).expect("happy path");

    // dataset file exists with 2 lines
    let dataset_body = fs::read_to_string(&dataset_path).expect("dataset readable");
    assert_eq!(dataset_body.lines().count(), 2);

    // manifest file exists, re-parses, and matches the run id
    let manifest_body = fs::read_to_string(&manifest_path).expect("manifest readable");
    let manifest_parsed: toml::Value = toml::from_str(&manifest_body).expect("valid toml");
    assert_eq!(manifest_parsed["run"]["id"].as_str(), Some("run:e2e:1"));
    assert_eq!(
        manifest_parsed["readiness"]["overall"].as_str(),
        Some("ready")
    );
    assert_eq!(
        manifest_parsed["dataset"]["trajectory_count"].as_integer(),
        Some(2)
    );

    // run agrees with the manifest
    assert_eq!(run.id().as_str(), "run:e2e:1");
    assert!(run.manifest().readiness().is_ready());
    assert_eq!(
        run.manifest().dataset().content_hash().as_str(),
        manifest_parsed["dataset"]["content_hash"].as_str().unwrap()
    );

    fs::remove_file(&dataset_path).ok();
    fs::remove_file(&manifest_path).ok();
}

#[test]
fn unready_manifest_is_persisted_before_run_is_refused() {
    let dataset_path = tmp("unready-ds", "jsonl");
    let manifest_path = tmp("unready-mf", "toml");

    let source = VecTrajectorySource {
        trajectories: vec![inspect_trajectory("traj:1", "read.inspect", "node:1")],
    };
    let dataset_writer = JsonlSftDatasetWriter::new(&dataset_path);
    let manifest_writer = TomlManifestWriter::new(&manifest_path);
    let use_case = BuildTrainingRunUseCase::new(source, dataset_writer, manifest_writer);

    let request = BuildTrainingRunRequest::new(
        TrainingRunId::parse("run:unready").unwrap(),
        TrainerTarget::new(
            TrainerCommand::parse("sft-trainer").unwrap(),
            BaseModelId::parse("base").unwrap(),
            OutputDirectory::parse("out").unwrap(),
        ),
        DatasetSource::parse("synthetic-run:test").unwrap(),
        // Threshold 10 but only 1 trajectory → gate fails.
        vec![ReadinessGate::MinimumTrajectories(
            PositiveCount::parse(10, "x").unwrap(),
        )],
        EvaluationReport::from_outcomes(Vec::new()),
    );

    let err = use_case.execute(&request).expect_err("must refuse run");
    assert!(matches!(
        err,
        operator_training_application::errors::training_application_error::TrainingApplicationError::Domain(
            operator_training_domain::errors::training_domain_error::TrainingDomainError::NotReady { .. }
        )
    ));

    // Even when the run is refused, the manifest lands on disk with
    // overall = not_ready — operators can inspect why.
    let manifest_body = fs::read_to_string(&manifest_path).expect("manifest readable");
    let manifest_parsed: toml::Value = toml::from_str(&manifest_body).expect("valid toml");
    assert_eq!(
        manifest_parsed["readiness"]["overall"].as_str(),
        Some("not_ready")
    );

    fs::remove_file(&dataset_path).ok();
    fs::remove_file(&manifest_path).ok();
}
