//! Filesystem round-trip tests for `TomlManifestWriter`. The exact
//! TOML byte layout is not asserted (a future serde-toml bump may
//! reformat whitespace); we re-parse the written file with the
//! `toml` crate and assert the parsed tree against expected values.

use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};

use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;
use operator_shared_domain::value_objects::positive_count::PositiveCount;
use operator_shared_domain::value_objects::task_family::TaskFamily;
use operator_training_application::ports::manifest_writer::ManifestWriter;
use operator_training_domain::ids::TrainingRunId;
use operator_training_domain::manifest::training_manifest::TrainingManifest;
use operator_training_domain::provenance::content_hash::ContentHash;
use operator_training_domain::provenance::dataset_provenance::DatasetProvenance;
use operator_training_domain::provenance::dataset_source::DatasetSource;
use operator_training_domain::provenance::task_family_distribution::TaskFamilyDistribution;
use operator_training_domain::provenance::task_family_distribution_entry::TaskFamilyDistributionEntry;
use operator_training_domain::readiness::pass_rate_percent::PassRatePercent;
use operator_training_domain::readiness::readiness_check::ReadinessCheck;
use operator_training_domain::readiness::readiness_gate::ReadinessGate;
use operator_training_domain::readiness::readiness_outcome::ReadinessOutcome;
use operator_training_domain::readiness::readiness_report::ReadinessReport;
use operator_training_domain::trainer::base_model_id::BaseModelId;
use operator_training_domain::trainer::output_directory::OutputDirectory;
use operator_training_domain::trainer::trainer_command::TrainerCommand;
use operator_training_domain::trainer::trainer_target::TrainerTarget;
use operator_training_infra::adapters::toml_manifest_writer::TomlManifestWriter;

static SEQ: AtomicU64 = AtomicU64::new(1);

fn tmp_path(label: &str) -> std::path::PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("operator-training-manifest-{label}-{pid}-{n}.toml"))
}

fn manifest_with(
    run_id: &str,
    distribution: TaskFamilyDistribution,
    readiness: ReadinessReport,
) -> TrainingManifest {
    let provenance = DatasetProvenance::new(
        DatasetSource::parse("synthetic-run:test").unwrap(),
        ContentHash::parse("sha256:abc").unwrap(),
        PositiveCount::parse(10, "trajectory_count").unwrap(),
        distribution,
    )
    .unwrap();
    let trainer_target = TrainerTarget::new(
        TrainerCommand::parse("sft-trainer").unwrap(),
        BaseModelId::parse("Qwen/Qwen2.5-1.5B-Instruct").unwrap(),
        OutputDirectory::parse("out/run").unwrap(),
    );
    TrainingManifest::new(
        TrainingRunId::parse(run_id).unwrap(),
        provenance,
        trainer_target,
        readiness,
    )
}

#[test]
fn writes_a_well_formed_toml_with_every_section() {
    let path = tmp_path("happy");
    let writer = TomlManifestWriter::new(&path);

    let distribution = TaskFamilyDistribution::new(vec![
        TaskFamilyDistributionEntry::new(
            TaskFamily::parse("read.inspect").unwrap(),
            PositiveCount::parse(6, "x").unwrap(),
        ),
        TaskFamilyDistributionEntry::new(
            TaskFamily::parse("read.ask").unwrap(),
            PositiveCount::parse(4, "x").unwrap(),
        ),
    ])
    .unwrap();

    let readiness = ReadinessReport::new(vec![
        ReadinessCheck::new(
            ReadinessGate::MinimumTrajectories(PositiveCount::parse(5, "x").unwrap()),
            ReadinessOutcome::Passed,
        ),
        ReadinessCheck::new(
            ReadinessGate::MinimumEvaluationPassRate(PassRatePercent::parse(0.9).unwrap()),
            ReadinessOutcome::Failed {
                reason: NonEmptyString::parse("rate 0.5 below target 0.9", "test").unwrap(),
            },
        ),
    ])
    .unwrap();

    let manifest = manifest_with("run:1", distribution, readiness);
    writer.write(&manifest).expect("write");

    let body = fs::read_to_string(&path).expect("readable");
    let parsed: toml::Value = toml::from_str(&body).expect("valid toml");

    assert_eq!(parsed["run"]["id"].as_str(), Some("run:1"));
    assert_eq!(
        parsed["dataset"]["source"].as_str(),
        Some("synthetic-run:test")
    );
    assert_eq!(
        parsed["dataset"]["content_hash"].as_str(),
        Some("sha256:abc")
    );
    assert_eq!(parsed["dataset"]["trajectory_count"].as_integer(), Some(10));
    let distribution_table = parsed["dataset"]["task_family_distribution"]
        .as_table()
        .expect("table");
    assert_eq!(distribution_table["read.inspect"].as_integer(), Some(6));
    assert_eq!(distribution_table["read.ask"].as_integer(), Some(4));

    assert_eq!(parsed["readiness"]["overall"].as_str(), Some("not_ready"));
    let gates = parsed["readiness"]["gate"].as_array().expect("array");
    assert_eq!(gates.len(), 2);
    assert_eq!(gates[0]["kind"].as_str(), Some("minimum_trajectories"));
    assert_eq!(gates[0]["status"].as_str(), Some("passed"));
    assert_eq!(gates[0]["target"].as_str(), Some("5"));
    assert_eq!(
        gates[1]["kind"].as_str(),
        Some("minimum_evaluation_pass_rate")
    );
    assert_eq!(gates[1]["status"].as_str(), Some("failed"));
    assert_eq!(gates[1]["target"].as_str(), Some("0.9000"));
    assert!(gates[1]["reason"].as_str().unwrap().contains("0.5"));

    assert_eq!(
        parsed["trainer_target"]["command"].as_str(),
        Some("sft-trainer")
    );
    assert_eq!(
        parsed["trainer_target"]["base_model"].as_str(),
        Some("Qwen/Qwen2.5-1.5B-Instruct")
    );
    assert_eq!(
        parsed["trainer_target"]["output_directory"].as_str(),
        Some("out/run")
    );

    fs::remove_file(&path).ok();
}

#[test]
fn ready_overall_when_every_gate_passes() {
    let path = tmp_path("ready");
    let writer = TomlManifestWriter::new(&path);
    let distribution = TaskFamilyDistribution::new(vec![]).unwrap();
    let readiness = ReadinessReport::new(vec![ReadinessCheck::new(
        ReadinessGate::MinimumTrajectories(PositiveCount::parse(1, "x").unwrap()),
        ReadinessOutcome::Passed,
    )])
    .unwrap();
    let manifest = manifest_with("run:ok", distribution, readiness);
    writer.write(&manifest).unwrap();

    let body = fs::read_to_string(&path).unwrap();
    let parsed: toml::Value = toml::from_str(&body).unwrap();
    assert_eq!(parsed["readiness"]["overall"].as_str(), Some("ready"));

    fs::remove_file(&path).ok();
}

#[test]
fn write_to_unwritable_path_surfaces_write_failure() {
    let writer = TomlManifestWriter::new("/this/path/does/not/exist/manifest.toml");
    let distribution = TaskFamilyDistribution::new(vec![]).unwrap();
    let readiness = ReadinessReport::new(vec![ReadinessCheck::new(
        ReadinessGate::MinimumTrajectories(PositiveCount::parse(1, "x").unwrap()),
        ReadinessOutcome::Passed,
    )])
    .unwrap();
    let manifest = manifest_with("run:1", distribution, readiness);

    let err = writer.write(&manifest).expect_err("must fail");
    match err {
        operator_training_application::errors::manifest_write_error::ManifestWriteError::WriteFailure {
            adapter,
            ..
        } => {
            assert_eq!(adapter, "toml_manifest_writer");
        }
        operator_training_application::errors::manifest_write_error::ManifestWriteError::SerialisationFailure { .. } => {
            panic!("expected WriteFailure, got SerialisationFailure");
        }
    }
}
