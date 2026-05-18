//! Operator training CLI. Wires the use cases in
//! `operator-training-application` to filesystem and process
//! adapters in `operator-training-infra` so the operator pipeline
//! can be driven from the command line.
//!
//! Subcommands:
//!
//! - `operator-train train` — invoke the Python trainer.
//! - `operator-train predict` — invoke the Python predictor.
//! - `operator-train validate` — score a trained adapter against a
//!   ground-truth holdout JSONL.
//! - `operator-train run` — `train` + `validate` (which itself
//!   invokes the predictor), fail fast at the first error.
//!
//! Exit codes:
//! - `0` — success (and, for `validate` / `run`, the readiness gate
//!   passed when `--min-pass-rate` was supplied).
//! - `1` — a use case returned an error, or the readiness gate failed.
//! - `2` — argument parsing failed (`clap` default).

use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, Subcommand};
use operator_shared_domain::contract::composite_action_contract_validator::CompositeActionContractValidator;
use operator_training_application::errors::training_application_error::TrainingApplicationError;
use operator_training_application::ports::predictor::Predictor;
use operator_training_application::ports::predictor_target::PredictorTarget;
use operator_training_application::ports::trainer_invoker::TrainerInvoker;
use operator_training_application::ports::trajectory_source::TrajectorySource;
use operator_training_application::use_cases::validate_trained_run_outcome::ValidateTrainedRunOutcome;
use operator_training_application::use_cases::validate_trained_run_request::ValidateTrainedRunRequest;
use operator_training_application::use_cases::validate_trained_run_use_case::ValidateTrainedRunUseCase;
use operator_training_domain::readiness::pass_rate_percent::PassRatePercent;
use operator_training_domain::trainer::base_model_id::BaseModelId;
use operator_training_domain::trainer::output_directory::OutputDirectory;
use operator_training_domain::trainer::trainer_command::TrainerCommand;
use operator_training_domain::trainer::trainer_target::TrainerTarget;
use operator_training_infra::adapters::composite_policy_evaluator::CompositePolicyEvaluator;
use operator_training_infra::adapters::filesystem_trajectory_source::FilesystemTrajectorySource;
use operator_training_infra::adapters::jsonl_predictions_reader_adapter::JsonlPredictionsReaderAdapter;
use operator_training_infra::adapters::process_predictor_invoker::ProcessPredictorInvoker;
use operator_training_infra::adapters::process_trainer_invoker::ProcessTrainerInvoker;

#[derive(Parser, Debug)]
#[command(
    name = "operator-train",
    about = "Drive the Operator training pipeline (train, predict, validate).",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Invoke the Python trainer script via `ProcessTrainerInvoker`.
    Train(TrainArgs),
    /// Invoke the Python predictor script via `ProcessPredictorInvoker`.
    Predict(PredictArgs),
    /// Score a trained adapter against a holdout, return pass/fail.
    Validate(ValidateArgs),
    /// `train` → `predict` → `validate` in one command.
    Run(RunArgs),
}

#[derive(clap::Args, Debug)]
struct TrainArgs {
    /// Path or command name of the trainer binary / script.
    #[arg(long)]
    command: String,
    /// `HuggingFace` base model id (e.g., `Qwen/Qwen2.5-0.5B-Instruct`).
    #[arg(long)]
    base_model: String,
    /// Where the trainer should write the `LoRA` adapter weights.
    #[arg(long)]
    output_dir: String,
}

#[derive(clap::Args, Debug)]
struct PredictArgs {
    /// Path or command name of the predictor binary / script.
    #[arg(long)]
    command: String,
    /// `HuggingFace` base model id used during inference.
    #[arg(long)]
    base_model: String,
    /// Directory containing the trained `LoRA` adapter weights.
    #[arg(long)]
    adapter_dir: String,
    /// JSONL holdout (conversational SFT shape) the predictor reads.
    #[arg(long)]
    dataset_jsonl: String,
    /// Where the predictor writes `predictions.jsonl` and `summary.json`.
    #[arg(long)]
    output_dir: String,
}

#[derive(clap::Args, Debug)]
struct ValidateArgs {
    /// Path to the predictor command / script.
    #[arg(long)]
    predictor_command: String,
    /// `HuggingFace` base model id.
    #[arg(long)]
    base_model: String,
    /// Directory containing the trained `LoRA` adapter weights.
    #[arg(long)]
    adapter_dir: String,
    /// JSONL holdout the predictor sees as input.
    #[arg(long)]
    dataset_jsonl: String,
    /// Where the predictor writes its output.
    #[arg(long)]
    predictor_output_dir: String,
    /// JSONL of `TrainingTrajectoryDto` rows used as ground truth
    /// for scoring; usually the same dataset the predictor ran on.
    #[arg(long)]
    ground_truth_jsonl: PathBuf,
    /// Minimum exact-match pass rate required for the verdict to be
    /// `PASS`. Omit to print metrics without gating.
    #[arg(long)]
    min_pass_rate: Option<f64>,
}

#[derive(clap::Args, Debug)]
struct RunArgs {
    // Trainer
    #[arg(long)]
    trainer_command: String,
    #[arg(long)]
    base_model: String,
    #[arg(long)]
    trainer_output_dir: String,
    // Predictor
    #[arg(long)]
    predictor_command: String,
    #[arg(long)]
    dataset_jsonl: String,
    #[arg(long)]
    predictor_output_dir: String,
    // Validation
    #[arg(long)]
    ground_truth_jsonl: PathBuf,
    #[arg(long)]
    min_pass_rate: Option<f64>,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        Command::Train(args) => match cmd_train(&args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("train failed: {err}");
                ExitCode::FAILURE
            }
        },
        Command::Predict(args) => match cmd_predict(&args) {
            Ok(()) => ExitCode::SUCCESS,
            Err(err) => {
                eprintln!("predict failed: {err}");
                ExitCode::FAILURE
            }
        },
        Command::Validate(args) => match cmd_validate(&args) {
            Ok(passed) => {
                if passed {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                }
            }
            Err(err) => {
                eprintln!("validate failed: {err}");
                ExitCode::FAILURE
            }
        },
        Command::Run(args) => match cmd_run(args) {
            Ok(passed) => {
                if passed {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                }
            }
            Err(err) => {
                eprintln!("run failed: {err}");
                ExitCode::FAILURE
            }
        },
    }
}

fn cmd_train(args: &TrainArgs) -> Result<(), CliError> {
    let target = make_trainer_target(&args.command, &args.base_model, &args.output_dir)?;
    let invoker = ProcessTrainerInvoker::new();
    let outcome = invoker
        .invoke(&target)
        .map_err(TrainingApplicationError::from)?;
    println!("trainer outcome: {outcome:?}");
    if outcome.is_success() {
        Ok(())
    } else {
        Err(CliError::Generic("trainer reported non-success".into()))
    }
}

fn cmd_predict(args: &PredictArgs) -> Result<(), CliError> {
    let target = make_predictor_target(
        &args.command,
        &args.base_model,
        &args.adapter_dir,
        &args.dataset_jsonl,
        &args.output_dir,
    )?;
    let invoker = ProcessPredictorInvoker::new();
    let outcome = invoker
        .predict(&target)
        .map_err(TrainingApplicationError::from)?;
    println!("predictions: {}", outcome.predictions());
    println!("failures:    {}", outcome.failures());
    println!("predictions_path: {}", outcome.predictions_path());
    println!("summary_path:     {}", outcome.summary_path());
    Ok(())
}

fn cmd_validate(args: &ValidateArgs) -> Result<bool, CliError> {
    let predictor_target = make_predictor_target(
        &args.predictor_command,
        &args.base_model,
        &args.adapter_dir,
        &args.dataset_jsonl,
        &args.predictor_output_dir,
    )?;
    // Read the ground truth eagerly so we fail fast on a bad file
    // before paying the predict cost. The use case does not consume
    // the source itself; it operates on the materialised vector.
    let ground_truth_source = FilesystemTrajectorySource::new(&args.ground_truth_jsonl);
    let ground_truth = ground_truth_source
        .fetch_all()
        .map_err(TrainingApplicationError::from)?;
    if ground_truth.is_empty() {
        return Err(CliError::Generic(
            "ground-truth JSONL is empty; nothing to validate".into(),
        ));
    }

    let predictions_path =
        std::path::PathBuf::from(args.predictor_output_dir.clone()).join("predictions.jsonl");
    let predictor = ProcessPredictorInvoker::new();
    let reader = JsonlPredictionsReaderAdapter::new(predictions_path);
    let validator = CompositeActionContractValidator::default_strict();
    let evaluator = CompositePolicyEvaluator::new(validator);

    let use_case = ValidateTrainedRunUseCase::new(predictor, reader, evaluator);
    let request = ValidateTrainedRunRequest::new(predictor_target, ground_truth);
    let outcome = use_case.execute(&request)?;

    print_validation_outcome(&outcome);

    match args.min_pass_rate {
        Some(rate) => {
            let threshold = PassRatePercent::parse(rate).map_err(|err| {
                CliError::Generic(format!("invalid --min-pass-rate {rate}: {err}"))
            })?;
            let passed = outcome.is_passing(threshold);
            println!("verdict: {}", if passed { "PASS" } else { "FAIL" });
            Ok(passed)
        }
        None => Ok(true),
    }
}

fn cmd_run(args: RunArgs) -> Result<bool, CliError> {
    // 1. Train
    let train_target = make_trainer_target(
        &args.trainer_command,
        &args.base_model,
        &args.trainer_output_dir,
    )?;
    let trainer = ProcessTrainerInvoker::new();
    let train_outcome = trainer
        .invoke(&train_target)
        .map_err(TrainingApplicationError::from)?;
    println!("[train] outcome: {train_outcome:?}");
    if !train_outcome.is_success() {
        return Err(CliError::Generic("trainer reported non-success".into()));
    }

    // 2. Validate (which itself invokes the predictor and scores).
    let validate_args = ValidateArgs {
        predictor_command: args.predictor_command,
        base_model: args.base_model,
        adapter_dir: args.trainer_output_dir,
        dataset_jsonl: args.dataset_jsonl,
        predictor_output_dir: args.predictor_output_dir,
        ground_truth_jsonl: args.ground_truth_jsonl,
        min_pass_rate: args.min_pass_rate,
    };
    cmd_validate(&validate_args)
}

fn make_trainer_target(
    command: &str,
    base_model: &str,
    output_dir: &str,
) -> Result<TrainerTarget, CliError> {
    Ok(TrainerTarget::new(
        TrainerCommand::parse(command).map_err(arg_err("--command"))?,
        BaseModelId::parse(base_model).map_err(arg_err("--base-model"))?,
        OutputDirectory::parse(output_dir).map_err(arg_err("--output-dir"))?,
    ))
}

fn make_predictor_target(
    command: &str,
    base_model: &str,
    adapter_dir: &str,
    dataset_jsonl: &str,
    output_dir: &str,
) -> Result<PredictorTarget, CliError> {
    PredictorTarget::new(
        TrainerCommand::parse(command).map_err(arg_err("--command"))?,
        BaseModelId::parse(base_model).map_err(arg_err("--base-model"))?,
        OutputDirectory::parse(adapter_dir).map_err(arg_err("--adapter-dir"))?,
        dataset_jsonl,
        OutputDirectory::parse(output_dir).map_err(arg_err("--output-dir"))?,
    )
    .map_err(arg_err("--*"))
}

fn arg_err(
    arg: &'static str,
) -> impl Fn(operator_shared_domain::error::domain_error::DomainError) -> CliError {
    move |err| CliError::Generic(format!("invalid {arg}: {err}"))
}

fn print_validation_outcome(outcome: &ValidateTrainedRunOutcome) {
    println!(
        "predictor.predictions: {}",
        outcome.predictor_outcome().predictions()
    );
    println!(
        "predictor.failures:    {}",
        outcome.predictor_outcome().failures()
    );
    println!(
        "evaluation.total:         {}",
        outcome.evaluation_report().total()
    );
    println!(
        "evaluation.exact_match:   {}",
        outcome.evaluation_report().exact_match_count()
    );
    println!(
        "evaluation.tool_match:    {}",
        outcome.evaluation_report().tool_match_count()
    );
    println!(
        "evaluation.contract_valid: {}",
        outcome.evaluation_report().contract_valid_count()
    );
    println!(
        "evaluation.exact_match_rate:    {:.4}",
        outcome.evaluation_report().exact_match_rate()
    );
    println!(
        "evaluation.tool_match_rate:     {:.4}",
        outcome.evaluation_report().tool_match_rate()
    );
    println!(
        "evaluation.contract_valid_rate: {:.4}",
        outcome.evaluation_report().contract_validity_rate()
    );
}

#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error(transparent)]
    Application(#[from] TrainingApplicationError),
    #[error("{0}")]
    Generic(String),
}
