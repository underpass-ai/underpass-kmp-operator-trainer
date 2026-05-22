//! `operator-realistic-corpus` — produce teacher-backed realistic
//! Operator trajectories from externally authored scenarios.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Parser;
use operator_shared_domain::action::escalate_action::EscalateAction;
use operator_shared_domain::action::escalate_reason::EscalateReason;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::action::stop_action::StopAction;
use operator_shared_domain::action::stop_reason::StopReason;
use operator_shared_domain::action::tool_call_action::ToolCallAction;
use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
use operator_shared_domain::tool_arguments::wake_arguments::WakeArguments;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::model_id::ModelId;
use operator_shared_domain::value_objects::positive_count::PositiveCount;
use operator_shared_infra::mappers::training_trajectory_mapper::TrainingTrajectoryMapper;
use operator_synthetic_application::error::teacher_policy_error::TeacherPolicyError;
use operator_synthetic_application::ports::scenario_source::ScenarioSource;
use operator_synthetic_application::ports::teacher_policy::TeacherPolicy;
use operator_synthetic_application::use_cases::build_realistic_corpus_use_case::BuildRealisticCorpusUseCase;
use operator_synthetic_application::use_cases::max_drop_rate::MaxDropRate;
use operator_synthetic_application::use_cases::realistic_corpus_report::RealisticCorpusReport;
use operator_synthetic_domain::calibration::calibration_subject::CalibrationSubject;
use operator_synthetic_infra::adapters::jsonl_scenario_source::JsonlScenarioSource;
use operator_synthetic_infra::adapters::openai_compatible_teacher_policy::OpenAiCompatibleTeacherPolicy;
use operator_synthetic_infra::dto::realistic_corpus_run_metadata_dto::RealisticCorpusRunMetadataDto;
use operator_synthetic_infra::mappers::realistic_corpus_report_mapper::RealisticCorpusReportMapper;
use reqwest::Url;
use sha2::{Digest, Sha256};

const PREDICTOR: &str = "operator-realistic-corpus-v1";
const STUB_ENV: &str = "OPERATOR_REALISTIC_CORPUS_STUB";

#[derive(Parser, Debug)]
#[command(
    name = "operator-realistic-corpus",
    about = "Build a realistic teacher-backed Operator corpus from external scenarios.",
    version
)]
struct Cli {
    #[arg(long)]
    scenarios: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long)]
    api_base: String,
    #[arg(long)]
    api_key_file: PathBuf,
    #[arg(long)]
    prompt: PathBuf,
    #[arg(long)]
    model: String,
    #[arg(long, default_value_t = 0.0)]
    temperature: f32,
    #[arg(long, default_value_t = 0.05)]
    max_drop_rate: f64,
    #[arg(long, default_value_t = 0)]
    limit: usize,
    #[arg(long)]
    validate_only: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(gate_passed) => {
            if gate_passed {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(err) => {
            eprintln!("realistic-corpus failed: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<bool, CliError> {
    let scenarios_count = precheck(cli)?;
    if cli.validate_only {
        eprintln!(
            "{{\"event\":\"validate_only\",\"scenarios_count\":{scenarios_count},\"status\":\"ok\"}}"
        );
        return Ok(true);
    }
    let started_at_unix = unix_now()?;
    let source: Box<dyn ScenarioSource> = Box::new(JsonlScenarioSource::new(&cli.scenarios));
    let teacher = teacher_policy(cli)?;
    let validator =
        operator_shared_domain::contract::composite_action_contract_validator::CompositeActionContractValidator::default_strict();
    let use_case = BuildRealisticCorpusUseCase::new(source, teacher, validator);
    let limit = if cli.limit == 0 {
        None
    } else {
        Some(
            PositiveCount::parse(cli.limit, "limit")
                .map_err(|err| CliError::Generic(format!("invalid --limit: {err}")))?,
        )
    };
    let max_drop_rate =
        MaxDropRate::parse(cli.max_drop_rate).map_err(|err| CliError::Generic(err.to_string()))?;
    let report = use_case
        .execute(max_drop_rate, limit)
        .map_err(|err| CliError::Generic(format!("build realistic corpus: {err}")))?;
    let finished_at_unix = unix_now()?;
    fs::create_dir_all(&cli.output).map_err(|err| {
        CliError::Generic(format!("create --output {}: {err}", cli.output.display()))
    })?;
    write_trajectories(&cli.output, &report)?;
    write_dropped(&cli.output, &report)?;
    write_report(cli, &report, started_at_unix, finished_at_unix)?;
    print_summary(&report, &cli.output);
    Ok(report.gate_passed())
}

fn precheck(cli: &Cli) -> Result<usize, CliError> {
    require_readable_non_empty(&cli.scenarios, "--scenarios")?;
    require_readable_non_empty(&cli.prompt, "--prompt")?;
    require_readable_non_empty(&cli.api_key_file, "--api-key-file")?;
    let key = read_key(&cli.api_key_file)?;
    if key.is_empty() {
        return Err(CliError::Generic(
            "--api-key-file is empty after trim".to_string(),
        ));
    }
    Url::parse(&cli.api_base)
        .map_err(|err| CliError::Generic(format!("invalid --api-base: {err}")))?;
    fs::create_dir_all(&cli.output).map_err(|err| {
        CliError::Generic(format!("create --output {}: {err}", cli.output.display()))
    })?;
    let scenarios = JsonlScenarioSource::new(&cli.scenarios)
        .read()
        .map_err(|err| CliError::Generic(format!("parse --scenarios: {err}")))?;
    Ok(scenarios.len())
}

fn teacher_policy(cli: &Cli) -> Result<Box<dyn TeacherPolicy>, CliError> {
    match std::env::var(STUB_ENV).ok().as_deref() {
        Some("accepted") => Ok(Box::new(StubTeacherPolicy::Accepted)),
        Some("wrong") => Ok(Box::new(StubTeacherPolicy::Wrong)),
        Some("shape") => Ok(Box::new(StubTeacherPolicy::Shape)),
        Some(other) => Err(CliError::Generic(format!(
            "unsupported {STUB_ENV} value '{other}'"
        ))),
        None => Ok(Box::new(
            OpenAiCompatibleTeacherPolicy::new(
                &cli.api_base,
                Some(read_key(&cli.api_key_file)?),
                &cli.model,
                &cli.prompt,
            )
            .map_err(|err| CliError::Generic(format!("build teacher adapter: {err}")))?
            .with_temperature(cli.temperature),
        )),
    }
}

fn write_trajectories(output: &Path, report: &RealisticCorpusReport) -> Result<(), CliError> {
    let path = output.join("trajectories.jsonl");
    let mut file = fs::File::create(&path)
        .map_err(|err| CliError::Generic(format!("create {}: {err}", path.display())))?;
    for row in report.accepted() {
        let dto = TrainingTrajectoryMapper::to_dto(row)
            .map_err(|err| CliError::Generic(format!("map trajectory: {err}")))?;
        let line = serde_json::to_string(&dto)
            .map_err(|err| CliError::Generic(format!("serialize trajectory: {err}")))?;
        writeln!(file, "{line}")
            .map_err(|err| CliError::Generic(format!("write {}: {err}", path.display())))?;
    }
    Ok(())
}

fn write_dropped(output: &Path, report: &RealisticCorpusReport) -> Result<(), CliError> {
    let path = output.join("dropped.jsonl");
    let mut file = fs::File::create(&path)
        .map_err(|err| CliError::Generic(format!("create {}: {err}", path.display())))?;
    for row in report.dropped() {
        let dto = RealisticCorpusReportMapper::drop_to_dto(row);
        let line = serde_json::to_string(&dto)
            .map_err(|err| CliError::Generic(format!("serialize drop entry: {err}")))?;
        writeln!(file, "{line}")
            .map_err(|err| CliError::Generic(format!("write {}: {err}", path.display())))?;
    }
    Ok(())
}

fn write_report(
    cli: &Cli,
    report: &RealisticCorpusReport,
    started_at_unix: u64,
    finished_at_unix: u64,
) -> Result<(), CliError> {
    let metadata = RealisticCorpusRunMetadataDto {
        predictor: PREDICTOR.to_string(),
        run_id: run_id_from_output(&cli.output),
        scenarios_path: cli.scenarios.display().to_string(),
        scenarios_sha256: sha256_file(&cli.scenarios)?,
        prompt_path: cli.prompt.display().to_string(),
        prompt_sha256: sha256_file(&cli.prompt)?,
        api_base: cli.api_base.clone(),
        model: cli.model.clone(),
        temperature: cli.temperature,
        started_at_unix,
        finished_at_unix,
    };
    let dto = RealisticCorpusReportMapper::to_dto(report, metadata);
    let path = cli.output.join("report.json");
    let payload = serde_json::to_string_pretty(&dto)
        .map_err(|err| CliError::Generic(format!("serialize report: {err}")))?;
    fs::write(&path, format!("{payload}\n"))
        .map_err(|err| CliError::Generic(format!("write {}: {err}", path.display())))
}

fn print_summary(report: &RealisticCorpusReport, output: &Path) {
    println!("predictor:       {PREDICTOR}");
    println!("total_scenarios: {}", report.total_scenarios());
    println!("accepted_count:  {}", report.accepted_count());
    println!("dropped_count:   {}", report.dropped_count());
    println!("drop_rate:       {:.4}", report.drop_rate());
    println!("gate_passed:     {}", report.gate_passed());
    if let Some(reason) = report.gate_failure_reason() {
        println!("gate_reason:     {reason}");
    }
    println!(
        "trajectories:    {}",
        output.join("trajectories.jsonl").display()
    );
    println!(
        "dropped:         {}",
        output.join("dropped.jsonl").display()
    );
    println!("report:          {}", output.join("report.json").display());
}

fn require_readable_non_empty(path: &Path, flag: &str) -> Result<(), CliError> {
    let metadata = fs::metadata(path).map_err(|err| {
        CliError::Generic(format!("{flag} {} is not readable: {err}", path.display()))
    })?;
    if !metadata.is_file() {
        return Err(CliError::Generic(format!(
            "{flag} {} is not a file",
            path.display()
        )));
    }
    if metadata.len() == 0 {
        return Err(CliError::Generic(format!(
            "{flag} {} is empty",
            path.display()
        )));
    }
    Ok(())
}

fn read_key(path: &Path) -> Result<String, CliError> {
    Ok(fs::read_to_string(path)
        .map_err(|err| CliError::Generic(format!("read api key {}: {err}", path.display())))?
        .trim()
        .to_string())
}

fn sha256_file(path: &Path) -> Result<String, CliError> {
    let bytes = fs::read(path)
        .map_err(|err| CliError::Generic(format!("read {} for sha256: {err}", path.display())))?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn unix_now() -> Result<u64, CliError> {
    Ok(SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| CliError::Generic(format!("system time before unix epoch: {err}")))?
        .as_secs())
}

fn run_id_from_output(path: &Path) -> String {
    path.file_name()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("realistic-v7")
        .to_string()
}

#[derive(Debug)]
enum StubTeacherPolicy {
    Accepted,
    Wrong,
    Shape,
}

impl TeacherPolicy for StubTeacherPolicy {
    fn decide(&self, subject: &CalibrationSubject) -> Result<OperatorAction, TeacherPolicyError> {
        match self {
            Self::Accepted => Ok(stub_action_for_subject(subject)),
            Self::Wrong => Ok(OperatorAction::ToolCall(ToolCallAction::new(
                ToolArguments::Wake(WakeArguments::new(subject.about().clone())),
            ))),
            Self::Shape => Err(TeacherPolicyError::Shape {
                adapter: "realistic_corpus_stub_teacher",
                message: "stubbed shape failure".to_string(),
            }),
        }
    }
}

fn stub_action_for_subject(subject: &CalibrationSubject) -> OperatorAction {
    if let Some(prepared) = subject.prepared_action() {
        return prepared.action().clone();
    }
    let family = subject.task_family().as_str();
    if family.contains("kernel_wake") {
        return OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Wake(
            WakeArguments::new(subject.about().clone()),
        )));
    }
    if family.contains("stop") {
        return OperatorAction::Stop(
            StopAction::new(
                StopReason::AnswerReady,
                Some("The evidence is sufficient.".to_string()),
                vec![first_ref(subject)],
            )
            .unwrap(),
        );
    }
    if family.contains("escalate") {
        return OperatorAction::Escalate(EscalateAction::new(
            EscalateReason::BeyondCapability,
            ModelId::parse("frontier-reasoner").unwrap(),
        ));
    }
    OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
        InspectArguments::new(first_ref(subject)),
    )))
}

fn first_ref(subject: &CalibrationSubject) -> MemoryRef {
    subject
        .visible_state()
        .known_refs()
        .iter()
        .next()
        .cloned()
        .unwrap_or_else(|| MemoryRef::parse("node:visible").unwrap())
}

#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error("{0}")]
    Generic(String),
}
