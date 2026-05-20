//! `operator-replay` — execute predicted actions against a live KMP
//! MCP JSON-RPC endpoint and report outcomes.
//!
//! Reads a `predictions.jsonl` produced by
//! `scripts/operator/predict_operator_sft.py` plus a ground-truth
//! trajectory JSONL (the same input you fed the predictor or the
//! holdout the trajectories were drawn from), joins them by
//! `step_id` to resolve `trajectory_id`, dispatches each predicted
//! action to the configured MCP endpoint via the real
//! `HttpKmpMcpClient`, and prints the resulting `ReplayReport`.
//!
//! Inputs use only domain types from `operator-shared-*` plus the
//! replay layers themselves — this CLI does **not** depend on
//! `operator-evaluation-*` per the bounded-context rule
//! `replay → shared`.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use operator_replay_application::use_cases::execute_replay_use_case::ExecuteReplayUseCase;
use operator_replay_domain::prediction::replay_prediction::ReplayPrediction;
use operator_replay_domain::report::replay_report::ReplayReport;
use operator_replay_infra::adapters::http_kmp_mcp_client::HttpKmpMcpClient;
use operator_shared_contract::operator_action_dto::OperatorActionDto;
use operator_shared_contract::training_trajectory_dto::TrainingTrajectoryDto;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::ids::step_id::StepId;
use operator_shared_domain::ids::training_trajectory_id::TrainingTrajectoryId;
use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;
use operator_shared_infra::mappers::operator_action_mapper::OperatorActionMapper;
use operator_shared_infra::mappers::training_trajectory_mapper::TrainingTrajectoryMapper;
use serde::Deserialize;

#[derive(Parser, Debug)]
#[command(
    name = "operator-replay",
    about = "Execute predicted actions against a live KMP MCP endpoint and report outcomes.",
    version
)]
struct Cli {
    /// `predictions.jsonl` produced by the Python predictor
    /// (`{step_id, action}` per line).
    #[arg(long)]
    predictions: PathBuf,
    /// JSONL of `TrainingTrajectoryDto` rows. Used to resolve
    /// `step_id` → `trajectory_id` so the report attributes each
    /// outcome to the right trajectory.
    #[arg(long)]
    ground_truth: PathBuf,
    /// Base URL of the kernel MCP JSON-RPC endpoint (e.g.,
    /// `http://localhost:8080`).
    #[arg(long)]
    mcp_endpoint: String,
    /// Minimum tool-call success rate (`successful / (successful +
    /// failed)`) required for the verdict to be `PASS`. Omit to
    /// print metrics only.
    #[arg(long)]
    min_success_rate: Option<f64>,
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
            eprintln!("replay failed: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<bool, CliError> {
    let predictions_rows = read_predictions(&cli.predictions)?;
    if predictions_rows.is_empty() {
        return Err(CliError::Generic(
            "predictions.jsonl is empty; nothing to replay".into(),
        ));
    }

    let ground_truth = read_trajectories_jsonl(&cli.ground_truth)?;
    if ground_truth.is_empty() {
        return Err(CliError::Generic(
            "ground-truth JSONL is empty; cannot resolve trajectory_ids".into(),
        ));
    }

    let trajectory_id_by_step: HashMap<StepId, TrainingTrajectoryId> = ground_truth
        .iter()
        .map(|t| (t.step_id().clone(), t.id().clone()))
        .collect();

    let mut replay_predictions = Vec::with_capacity(predictions_rows.len());
    for (step_id, action) in predictions_rows {
        let Some(trajectory_id) = trajectory_id_by_step.get(&step_id) else {
            eprintln!(
                "warn: prediction for step `{}` has no matching ground-truth trajectory; skipping",
                step_id.as_str()
            );
            continue;
        };
        replay_predictions.push(ReplayPrediction::new(trajectory_id.clone(), action));
    }
    if replay_predictions.is_empty() {
        return Err(CliError::Generic(
            "no predictions overlap the ground-truth set; check step_ids".into(),
        ));
    }

    let client = HttpKmpMcpClient::new(cli.mcp_endpoint.clone())
        .map_err(|err| CliError::Generic(format!("build MCP client: {err}")))?;
    let use_case = ExecuteReplayUseCase::new(client);
    let report = use_case
        .execute(&replay_predictions)
        .map_err(|err| CliError::Generic(format!("execute replay: {err}")))?;

    print_report(&report);

    match cli.min_success_rate {
        Some(rate) => {
            let actual = report.tool_call_success_rate().unwrap_or(0.0);
            let passed = actual >= rate;
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

#[derive(Debug, Deserialize)]
struct PredictionRowDto {
    step_id: String,
    action: OperatorActionDto,
}

fn read_predictions(path: &std::path::Path) -> Result<Vec<(StepId, OperatorAction)>, CliError> {
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
        let dto: PredictionRowDto = serde_json::from_str(&line).map_err(|err| {
            CliError::Generic(format!("parse predictions line {}: {err}", index + 1))
        })?;
        let step_id = StepId::parse(dto.step_id.clone()).map_err(|err| {
            CliError::Generic(format!(
                "predictions line {} step_id `{}`: {err}",
                index + 1,
                dto.step_id
            ))
        })?;
        let action = OperatorActionMapper::to_domain(&dto.action).map_err(|err| {
            CliError::Generic(format!("predictions line {} action: {err}", index + 1))
        })?;
        out.push((step_id, action));
    }
    Ok(out)
}

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
        let dto: TrainingTrajectoryDto = serde_json::from_str(&line).map_err(|err| {
            CliError::Generic(format!("parse ground-truth line {}: {err}", index + 1))
        })?;
        let trajectory = TrainingTrajectoryMapper::to_domain(&dto).map_err(|err| {
            CliError::Generic(format!(
                "map ground-truth line {} to domain: {err}",
                index + 1
            ))
        })?;
        out.push(trajectory);
    }
    Ok(out)
}

fn print_report(report: &ReplayReport) {
    println!("total:                   {}", report.total());
    println!(
        "successful_tool_calls:   {}",
        report.successful_tool_calls()
    );
    println!("failed_tool_calls:       {}", report.failed_tool_calls());
    println!("stop_or_escalate:        {}", report.stop_or_escalate());
    match report.tool_call_success_rate() {
        Some(rate) => println!("tool_call_success_rate:  {rate:.4}"),
        None => println!("tool_call_success_rate:  n/a (no tool calls dispatched)"),
    }
}

#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error("{0}")]
    Generic(String),
}
