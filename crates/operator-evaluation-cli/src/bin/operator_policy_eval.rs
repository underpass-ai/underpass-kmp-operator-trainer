//! `operator-policy-eval` — score predictions against ground truth.
//!
//! Reads a `predictions.jsonl` (output of
//! `scripts/operator/predict_operator_sft.py`) plus a ground-truth
//! JSONL of `TrainingTrajectoryDto` rows, joins them by `step_id`,
//! scores each pair through `EvaluateOperatorPolicyUseCase` with the
//! strict KMP action contract, prints per-tool / per-mode metrics,
//! and (with `--min-pass-rate`) exits 0/1 based on a pass-rate gate.
//!
//! Useful for inspecting a historical run without re-executing the
//! predictor.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use operator_evaluation_application::use_cases::evaluate_operator_policy_use_case::EvaluateOperatorPolicyUseCase;
use operator_evaluation_domain::prediction::step_prediction_join::join_step_predictions;
use operator_evaluation_domain::report::evaluation_report::EvaluationReport;
use operator_evaluation_infra::adapters::jsonl_predictions_reader::JsonlPredictionsReader;
use operator_shared_contract::training_trajectory_dto::TrainingTrajectoryDto;
use operator_shared_domain::contract::composite_action_contract_validator::CompositeActionContractValidator;
use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;
use operator_shared_infra::mappers::training_trajectory_mapper::TrainingTrajectoryMapper;

#[derive(Parser, Debug)]
#[command(
    name = "operator-policy-eval",
    about = "Score a predictions.jsonl against a ground-truth JSONL.",
    version
)]
struct Cli {
    /// `predictions.jsonl` written by the Python predictor.
    #[arg(long)]
    predictions: PathBuf,
    /// JSONL of `TrainingTrajectoryDto` rows used as ground truth.
    #[arg(long)]
    ground_truth: PathBuf,
    /// Minimum exact-match rate required for the verdict to be
    /// `PASS`. Omit to print metrics without gating.
    #[arg(long)]
    min_pass_rate: Option<f64>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(passed) => {
            if passed {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(err) => {
            eprintln!("policy-eval failed: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<bool, CliError> {
    let predictions = JsonlPredictionsReader::new(&cli.predictions)
        .read()
        .map_err(|err| CliError::Generic(format!("read predictions: {err}")))?;
    if predictions.parsed().is_empty() && predictions.shape_violations().is_empty() {
        return Err(CliError::Generic(
            "predictions.jsonl is empty; nothing to score".into(),
        ));
    }

    let ground_truth = read_trajectories_jsonl(&cli.ground_truth)?;
    if ground_truth.is_empty() {
        return Err(CliError::Generic(
            "ground-truth JSONL is empty; nothing to score".into(),
        ));
    }

    let pairs = join_step_predictions(&ground_truth, predictions.parsed());
    if !predictions.parsed().is_empty() && pairs.is_empty() {
        return Err(CliError::Generic(
            "no predictions overlap the ground-truth set; check step_ids".into(),
        ));
    }

    let evaluator =
        EvaluateOperatorPolicyUseCase::new(CompositeActionContractValidator::default_strict());
    let report = evaluator
        .execute(&pairs, predictions.shape_violations())
        .map_err(|err| CliError::Generic(format!("evaluate: {err}")))?;

    print_report(&report);

    match cli.min_pass_rate {
        Some(rate) => {
            let passed = report.exact_match_rate() >= rate;
            println!(
                "verdict: {} (threshold {:.4})",
                if passed { "PASS" } else { "FAIL" },
                rate
            );
            Ok(passed)
        }
        None => Ok(true),
    }
}

/// Read `TrainingTrajectoryDto` rows from a JSONL file and map them
/// into domain `TrainingTrajectory` values. Skips blank lines. Kept
/// inline (not as a shared `training-infra` adapter) so this CLI
/// stays within the `evaluation → shared` dependency edge.
fn read_trajectories_jsonl(path: &std::path::Path) -> Result<Vec<TrainingTrajectory>, CliError> {
    let file = File::open(path)
        .map_err(|err| CliError::Generic(format!("open {}: {err}", path.display())))?;
    let reader = BufReader::new(file);
    let mut out = Vec::new();
    for (index, line) in reader.lines().enumerate() {
        let line =
            line.map_err(|err| CliError::Generic(format!("read line {}: {err}", index + 1)))?;
        if line.trim().is_empty() {
            continue;
        }
        let dto: TrainingTrajectoryDto = serde_json::from_str(&line)
            .map_err(|err| CliError::Generic(format!("parse line {}: {err}", index + 1)))?;
        let trajectory = TrainingTrajectoryMapper::to_domain(&dto)
            .map_err(|err| CliError::Generic(format!("map line {} to domain: {err}", index + 1)))?;
        out.push(trajectory);
    }
    Ok(out)
}

fn print_report(report: &EvaluationReport) {
    println!("total:               {}", report.total());
    println!("parsed:              {}", report.parsed_count());
    println!(
        "shape_invalid:       {} ({:.2}%)",
        report.shape_invalid_count(),
        report.shape_invalid_rate() * 100.0
    );
    println!("exact_match:         {}", report.exact_match_count());
    println!("tool_match:          {}", report.tool_match_count());
    println!("contract_valid:      {}", report.contract_valid_count());
    println!("exact_match_rate:    {:.4}", report.exact_match_rate());
    println!("tool_match_rate:     {:.4}", report.tool_match_rate());
    println!(
        "contract_valid_rate: {:.4}",
        report.contract_validity_rate()
    );
    for metric in report.per_tool() {
        let label = metric
            .tool()
            .map_or_else(|| "<stop/escalate>".to_string(), |t| t.as_str().to_string());
        println!(
            "  [{label}] total={} exact={} tool={} valid={}",
            metric.total(),
            metric.exact_matches(),
            metric.tool_matches(),
            metric.contract_valid(),
        );
    }
}

#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error("{0}")]
    Generic(String),
}
