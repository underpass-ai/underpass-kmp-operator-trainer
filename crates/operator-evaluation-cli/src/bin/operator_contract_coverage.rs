//! `operator-contract-coverage` — audit a trajectory JSONL for KMP
//! action contract coverage and validity.
//!
//! Reads a JSONL of `TrainingTrajectoryDto` rows, classifies each
//! trajectory's `target_action` by `OperatorActionKind` + `KernelTool`
//! (when applicable), counts per-mode breakdowns, and validates every
//! action against `CompositeActionContractValidator::default_strict()`
//! using the trajectory's own mode + visible state.
//!
//! Optional gates:
//! - `--require-full-coverage` — exit 1 if any `KernelTool` variant
//!   has zero trajectories.
//! - `--require-zero-invalid` — exit 1 if any trajectory's action
//!   violates the strict contract.
//!
//! Without either gate the CLI prints metrics and exits 0.

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use operator_shared_contract::training_trajectory_dto::TrainingTrajectoryDto;
use operator_shared_domain::action::operator_action::OperatorActionKind;
use operator_shared_domain::contract::action_contract_validator::ActionContractValidator;
use operator_shared_domain::contract::composite_action_contract_validator::CompositeActionContractValidator;
use operator_shared_domain::mode::operator_mode::OperatorMode;
use operator_shared_domain::tool::kernel_tool::KernelTool;
use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;
use operator_shared_infra::mappers::training_trajectory_mapper::TrainingTrajectoryMapper;

#[derive(Parser, Debug)]
#[command(
    name = "operator-contract-coverage",
    about = "Audit a trajectory JSONL for KMP action contract coverage and validity.",
    version
)]
struct Cli {
    /// JSONL of `TrainingTrajectoryDto` rows to audit.
    #[arg(long)]
    trajectories: PathBuf,
    /// Exit non-zero if any `KernelTool` variant has zero
    /// trajectories. Useful to gate "every capability covered" in CI.
    #[arg(long, default_value_t = false)]
    require_full_coverage: bool,
    /// Exit non-zero if any trajectory's action fails the strict
    /// action contract. Useful to gate "no malformed trajectory
    /// ships" in CI.
    #[arg(long, default_value_t = false)]
    require_zero_invalid: bool,
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
            eprintln!("contract-coverage failed: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<bool, CliError> {
    let trajectories = read_trajectories(&cli.trajectories)?;
    if trajectories.is_empty() {
        return Err(CliError::Generic(
            "trajectories JSONL is empty; nothing to audit".into(),
        ));
    }

    let validator = CompositeActionContractValidator::default_strict();
    let report = compute_report(&trajectories, &validator);
    print_report(&report);

    let mut passed = true;
    if cli.require_full_coverage && !report.has_full_tool_coverage() {
        passed = false;
        eprintln!(
            "fail: --require-full-coverage: {} of {} `KernelTool` variants uncovered",
            KernelTool::ALL.len() - report.distinct_tools(),
            KernelTool::ALL.len(),
        );
    }
    if cli.require_zero_invalid && report.invalid > 0 {
        passed = false;
        eprintln!(
            "fail: --require-zero-invalid: {} trajectories violate the strict contract",
            report.invalid,
        );
    }
    Ok(passed)
}

#[derive(Debug, Default)]
struct CoverageReport {
    total: usize,
    tool_call: usize,
    stop: usize,
    escalate: usize,
    valid: usize,
    invalid: usize,
    per_tool: BTreeMap<&'static str, usize>,
    per_mode: BTreeMap<&'static str, usize>,
}

impl CoverageReport {
    fn distinct_tools(&self) -> usize {
        self.per_tool.values().filter(|count| **count > 0).count()
    }

    fn has_full_tool_coverage(&self) -> bool {
        self.distinct_tools() == KernelTool::ALL.len()
    }
}

fn compute_report<V: ActionContractValidator>(
    trajectories: &[TrainingTrajectory],
    validator: &V,
) -> CoverageReport {
    let mut report = CoverageReport::default();
    for tool in KernelTool::ALL {
        report.per_tool.insert(tool.as_str(), 0);
    }
    report.per_mode.insert(OperatorMode::Read.as_str(), 0);
    report.per_mode.insert(OperatorMode::Write.as_str(), 0);

    for trajectory in trajectories {
        report.total += 1;
        let action = trajectory.target_action();
        match action.kind() {
            OperatorActionKind::ToolCall => {
                report.tool_call += 1;
                if let Some(tool) = action.tool() {
                    *report.per_tool.entry(tool.as_str()).or_insert(0) += 1;
                }
            }
            OperatorActionKind::Stop => report.stop += 1,
            OperatorActionKind::Escalate => report.escalate += 1,
        }
        *report
            .per_mode
            .entry(trajectory.mode().as_str())
            .or_insert(0) += 1;
        match validator.validate(action, trajectory.mode(), trajectory.visible_state()) {
            Ok(()) => report.valid += 1,
            Err(_) => report.invalid += 1,
        }
    }
    report
}

fn print_report(report: &CoverageReport) {
    println!("total:               {}", report.total);
    println!("tool_call:           {}", report.tool_call);
    println!("stop:                {}", report.stop);
    println!("escalate:            {}", report.escalate);
    println!("valid:               {}", report.valid);
    println!("invalid:             {}", report.invalid);
    let covered = report.distinct_tools();
    let total_tools = KernelTool::ALL.len();
    #[allow(clippy::cast_precision_loss)]
    let rate = covered as f64 / total_tools as f64;
    println!("tool_coverage_ratio: {covered}/{total_tools} ({rate:.4})");
    println!("per-tool:");
    for tool in KernelTool::ALL {
        let count = report.per_tool.get(tool.as_str()).copied().unwrap_or(0);
        let marker = if count > 0 { "ok" } else { "uncovered" };
        println!("  [{}] {count} ({marker})", tool.as_str());
    }
    println!("per-mode:");
    for (mode_name, count) in &report.per_mode {
        println!("  [{mode_name}] {count}");
    }
}

fn read_trajectories(path: &std::path::Path) -> Result<Vec<TrainingTrajectory>, CliError> {
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

#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error("{0}")]
    Generic(String),
}
