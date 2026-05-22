//! `operator-synthesize` — generate a synthetic `TrainingTrajectory`
//! dataset and write it as JSONL the rest of the operator pipeline
//! consumes (`operator-train`, `operator-policy-eval`,
//! `operator-replay`).
//!
//! This first cut wires the canonical default blueprint —
//! `SyntheticDatasetBlueprint::for_all_capabilities` — against the
//! `InMemorySyntheticCaseGenerator`. The in-memory generator is a
//! fixture-grade source (one canonical trajectory per
//! `KmpMcpCapability`, cloned N times). It satisfies the strict KMP
//! action contract but does not reflect realistic operator behaviour;
//! the teacher-backed generator lives behind the same application port
//! and is wired by later corpus-production flows.
//!
//! The CLI writes the dataset as `TrainingTrajectoryDto` JSONL (one
//! row per line) and prints a per-case coverage report. Output is
//! deterministic given the same `--dataset-id` and `--minimum-examples`.

use std::fs;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use operator_shared_domain::ids::dataset_id::DatasetId;
use operator_shared_domain::value_objects::positive_count::PositiveCount;
use operator_shared_infra::mappers::training_trajectory_mapper::TrainingTrajectoryMapper;
use operator_synthetic_application::use_cases::generate_synthetic_dataset_use_case::GenerateSyntheticDatasetUseCase;
use operator_synthetic_domain::dataset::synthetic_dataset_blueprint::SyntheticDatasetBlueprint;
use operator_synthetic_domain::dataset::synthetic_dataset_generation_report::SyntheticDatasetGenerationReport;
use operator_synthetic_infra::generators::in_memory_synthetic_case_generator::InMemorySyntheticCaseGenerator;

#[derive(Parser, Debug)]
#[command(
    name = "operator-synthesize",
    about = "Generate a synthetic TrainingTrajectory JSONL dataset for the operator pipeline.",
    version
)]
struct Cli {
    /// Identifier the generated `SyntheticDataset` advertises (e.g.,
    /// `dataset:smoke-2026-05-20`). Must be non-empty.
    #[arg(long)]
    dataset_id: String,
    /// Minimum examples to generate per `KmpMcpCapability`. Must be
    /// `> 0`. The total trajectory count is
    /// `minimum-examples × number-of-capabilities`.
    #[arg(long)]
    minimum_examples: usize,
    /// Path of the JSONL file to write. Parent directories are
    /// created if missing. An existing file is overwritten.
    #[arg(long)]
    output: PathBuf,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("synthesize failed: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<(), CliError> {
    let dataset_id = DatasetId::parse(cli.dataset_id.clone())
        .map_err(|err| CliError::Generic(format!("invalid --dataset-id: {err}")))?;
    let minimum_examples = PositiveCount::parse(cli.minimum_examples, "minimum_examples")
        .map_err(|err| CliError::Generic(format!("invalid --minimum-examples: {err}")))?;

    let blueprint = SyntheticDatasetBlueprint::for_all_capabilities(dataset_id, minimum_examples)
        .map_err(|err| CliError::Generic(format!("build blueprint: {err}")))?;
    let use_case = GenerateSyntheticDatasetUseCase::new(InMemorySyntheticCaseGenerator::new());
    let report = use_case
        .execute(&blueprint)
        .map_err(|err| CliError::Generic(format!("generate dataset: {err}")))?;

    write_jsonl(&cli.output, &report)?;
    print_report(&report, &cli.output);
    Ok(())
}

fn write_jsonl(
    path: &std::path::Path,
    report: &SyntheticDatasetGenerationReport,
) -> Result<(), CliError> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|err| {
            CliError::Generic(format!("create parent {}: {err}", parent.display()))
        })?;
    }
    let file = fs::File::create(path)
        .map_err(|err| CliError::Generic(format!("create {}: {err}", path.display())))?;
    let mut writer = BufWriter::new(file);
    for trajectory in report.dataset().trajectories() {
        let dto = TrainingTrajectoryMapper::to_dto(trajectory)
            .map_err(|err| CliError::Generic(format!("map trajectory to DTO: {err}")))?;
        let line = serde_json::to_string(&dto)
            .map_err(|err| CliError::Generic(format!("serialise trajectory: {err}")))?;
        writer
            .write_all(line.as_bytes())
            .map_err(|err| CliError::Generic(format!("write trajectory: {err}")))?;
        writer
            .write_all(b"\n")
            .map_err(|err| CliError::Generic(format!("write newline: {err}")))?;
    }
    writer
        .flush()
        .map_err(|err| CliError::Generic(format!("flush: {err}")))?;
    let file = writer
        .into_inner()
        .map_err(|err| CliError::Generic(format!("unwrap BufWriter: {}", err.error())))?;
    file.sync_all()
        .map_err(|err| CliError::Generic(format!("sync_all: {err}")))?;
    Ok(())
}

fn print_report(report: &SyntheticDatasetGenerationReport, output: &std::path::Path) {
    let dataset = report.dataset();
    println!("dataset_id:        {}", dataset.dataset_id().as_str());
    println!("trajectory_count:  {}", dataset.trajectories().len());
    println!("output:            {}", output.display());
    println!("per-case coverage:");
    for metric in report.case_metrics() {
        let satisfied = if metric.satisfies_minimum() {
            "ok"
        } else {
            "under"
        };
        println!(
            "  [{}] minimum={} generated={} {}",
            metric.case_id().as_str(),
            metric.expected_minimum().as_usize(),
            metric.generated().as_usize(),
            satisfied,
        );
    }
    if !report.every_case_satisfies_minimum() {
        println!("warning: at least one case under-generated; downstream gates may fail");
    }
}

#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error("{0}")]
    Generic(String),
}
