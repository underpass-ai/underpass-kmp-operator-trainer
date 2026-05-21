//! `operator-teacher-calibration` — score a teacher model against
//! hand-authored Operator calibration cases.

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
use operator_shared_domain::cursor::cursor::Cursor;
use operator_shared_domain::cursor::ref_cursor::RefCursor;
use operator_shared_domain::tool_arguments::ask_arguments::AskArguments;
use operator_shared_domain::tool_arguments::forward_arguments::ForwardArguments;
use operator_shared_domain::tool_arguments::goto_arguments::GotoArguments;
use operator_shared_domain::tool_arguments::ingest_arguments::IngestArguments;
use operator_shared_domain::tool_arguments::ingest_dimension::IngestDimension;
use operator_shared_domain::tool_arguments::ingest_entry::IngestEntry;
use operator_shared_domain::tool_arguments::ingest_memory::IngestMemory;
use operator_shared_domain::tool_arguments::ingest_temporal_coordinate::IngestTemporalCoordinate;
use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
use operator_shared_domain::tool_arguments::near_arguments::NearArguments;
use operator_shared_domain::tool_arguments::rewind_arguments::RewindArguments;
use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
use operator_shared_domain::tool_arguments::trace_arguments::TraceArguments;
use operator_shared_domain::tool_arguments::wake_arguments::WakeArguments;
use operator_shared_domain::tool_arguments::write_memory_arguments::WriteMemoryArguments;
use operator_shared_domain::value_objects::dimension_ref::DimensionRef;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::model_id::ModelId;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;
use operator_shared_domain::value_objects::positive_count::PositiveCount;
use operator_shared_domain::value_objects::string_map::StringMap;
use operator_synthetic_application::error::teacher_policy_error::TeacherPolicyError;
use operator_synthetic_application::ports::calibration_episode_source::CalibrationEpisodeSource;
use operator_synthetic_application::ports::teacher_policy::TeacherPolicy;
use operator_synthetic_application::use_cases::evaluate_teacher_calibration_use_case::EvaluateTeacherCalibrationUseCase;
use operator_synthetic_domain::calibration::calibration_subject::CalibrationSubject;
use operator_synthetic_infra::adapters::jsonl_calibration_episode_source::JsonlCalibrationEpisodeSource;
use operator_synthetic_infra::adapters::openai_compatible_teacher_policy::OpenAiCompatibleTeacherPolicy;
use operator_synthetic_infra::dto::teacher_calibration_run_metadata_dto::TeacherCalibrationRunMetadataDto;
use operator_synthetic_infra::mappers::teacher_calibration_report_mapper::TeacherCalibrationReportMapper;
use reqwest::Url;
use sha2::{Digest, Sha256};

const PREDICTOR: &str = "operator-teacher-calibration-v1";
const STUB_ENV: &str = "OPERATOR_TEACHER_CALIBRATION_STUB";

#[derive(Parser, Debug)]
#[command(
    name = "operator-teacher-calibration",
    about = "Evaluate a teacher model against Operator KMP/MCP calibration cases.",
    version
)]
struct Cli {
    #[arg(long)]
    cases: PathBuf,
    #[arg(long)]
    prompt: PathBuf,
    #[arg(long)]
    api_base: String,
    #[arg(long)]
    api_key_file: PathBuf,
    #[arg(long)]
    model: String,
    #[arg(long, default_value_t = 0.0)]
    temperature: f32,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value_t = 0)]
    limit: usize,
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
            eprintln!("teacher-calibration failed: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<bool, CliError> {
    precheck(cli)?;
    let started_at_unix = unix_now()?;
    let source: Box<dyn CalibrationEpisodeSource> =
        Box::new(JsonlCalibrationEpisodeSource::new(&cli.cases).with_limit(cli.limit));
    let teacher = teacher_policy(cli)?;
    let use_case = EvaluateTeacherCalibrationUseCase::new(source, teacher);
    let report = use_case
        .execute()
        .map_err(|err| CliError::Generic(format!("evaluate calibration: {err}")))?;
    let finished_at_unix = unix_now()?;
    let metadata = TeacherCalibrationRunMetadataDto {
        predictor: PREDICTOR.to_string(),
        dataset_path: cli.cases.display().to_string(),
        dataset_sha256: sha256_file(&cli.cases)?,
        prompt_path: cli.prompt.display().to_string(),
        prompt_sha256: sha256_file(&cli.prompt)?,
        api_base: cli.api_base.clone(),
        model: cli.model.clone(),
        temperature: cli.temperature,
        started_at_unix,
        finished_at_unix,
    };
    let dto = TeacherCalibrationReportMapper::to_dto(&report, metadata);
    write_report(&cli.output, &dto)?;
    print_summary(&dto, &cli.output);
    Ok(dto.gate_passed)
}

fn precheck(cli: &Cli) -> Result<(), CliError> {
    require_readable_non_empty(&cli.cases, "--cases")?;
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
    Ok(())
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

fn write_report(
    output: &Path,
    dto: &operator_synthetic_infra::dto::teacher_calibration_report_dto::TeacherCalibrationReportDto,
) -> Result<(), CliError> {
    let report_path = output.join("report.json");
    let mut file = fs::File::create(&report_path)
        .map_err(|err| CliError::Generic(format!("create {}: {err}", report_path.display())))?;
    let payload = serde_json::to_string_pretty(dto)
        .map_err(|err| CliError::Generic(format!("serialize report: {err}")))?;
    file.write_all(payload.as_bytes())
        .map_err(|err| CliError::Generic(format!("write report: {err}")))?;
    file.write_all(b"\n")
        .map_err(|err| CliError::Generic(format!("write report newline: {err}")))?;
    file.sync_all()
        .map_err(|err| CliError::Generic(format!("sync report: {err}")))?;
    Ok(())
}

fn print_summary(
    dto: &operator_synthetic_infra::dto::teacher_calibration_report_dto::TeacherCalibrationReportDto,
    output: &Path,
) {
    println!("predictor:            {}", dto.predictor);
    println!("model:                {}", dto.model);
    println!("total_cases:          {}", dto.total_cases);
    println!("match_count:          {}", dto.match_count);
    println!("tool_match_count:     {}", dto.tool_match_count);
    println!("contract_valid_count: {}", dto.contract_valid_count);
    println!("shape_failed_count:   {}", dto.shape_failed_count);
    println!("overall_accuracy:     {:.4}", dto.overall_accuracy);
    if let Some(value) = dto.per_category_accuracy.get("happy").and_then(|v| *v) {
        println!("happy_accuracy:       {value:.4}");
    }
    if let Some(value) = dto
        .per_category_accuracy
        .get("adversarial")
        .and_then(|v| *v)
    {
        println!("adversarial_accuracy: {value:.4}");
    }
    println!("gate_passed:          {}", dto.gate_passed);
    if let Some(reason) = dto.gate_failure_reason.as_deref() {
        println!("gate_failure_reason:  {reason}");
    }
    println!(
        "report:               {}",
        output.join("report.json").display()
    );
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
            Self::Wrong => Ok(OperatorAction::Escalate(EscalateAction::new(
                EscalateReason::BeyondCapability,
                ModelId::parse("frontier-reasoner").unwrap(),
            ))),
            Self::Shape => Err(TeacherPolicyError::Shape {
                adapter: "stub_teacher_policy",
                message: "stubbed shape failure".to_string(),
            }),
        }
    }
}

fn stub_action_for_subject(subject: &CalibrationSubject) -> OperatorAction {
    let family = subject.task_family().as_str();
    if family.contains("kernel_ingest") {
        return ingest(subject);
    }
    if family.contains("kernel_wake") {
        return tool(ToolArguments::Wake(WakeArguments::new(
            subject.about().clone(),
        )));
    }
    if family.contains("kernel_ask") {
        return tool(ToolArguments::Ask(
            AskArguments::new("What is known now?").unwrap(),
        ));
    }
    if family.contains("kernel_near") {
        return tool(ToolArguments::Near(
            NearArguments::new(first_ref(subject), vec![first_dim(subject)], Some(count(3)))
                .unwrap(),
        ));
    }
    if family.contains("kernel_goto") {
        return tool(ToolArguments::Goto(GotoArguments::new(Cursor::Ref(
            RefCursor::new(first_ref(subject)),
        ))));
    }
    if family.contains("kernel_rewind") {
        let Cursor::Temporal(cursor) = subject.visible_state().active_cursor().unwrap().clone()
        else {
            panic!("stub rewind subject requires temporal cursor");
        };
        return tool(ToolArguments::Rewind(RewindArguments::new(
            cursor,
            count(2),
        )));
    }
    if family.contains("kernel_forward") {
        let Cursor::Temporal(cursor) = subject.visible_state().active_cursor().unwrap().clone()
        else {
            panic!("stub forward subject requires temporal cursor");
        };
        return tool(ToolArguments::Forward(ForwardArguments::new(
            cursor,
            count(2),
        )));
    }
    if family.contains("kernel_trace") {
        return tool(ToolArguments::Trace(TraceArguments::new(
            first_ref(subject),
            Some(second_ref(subject)),
            count(8),
        )));
    }
    if family.contains("kernel_write_memory") {
        return tool(ToolArguments::WriteMemory(
            WriteMemoryArguments::new(
                "Record calibrated memory.",
                "The calibrated write is ready.",
                vec![first_ref(subject)],
            )
            .unwrap(),
        ));
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
    tool(ToolArguments::Inspect(InspectArguments::new(first_ref(
        subject,
    ))))
}

fn ingest(subject: &CalibrationSubject) -> OperatorAction {
    let dimension = first_dim(subject);
    let entry = MemoryRef::parse("node:calibration:new").unwrap();
    let coordinate = IngestTemporalCoordinate::new(
        dimension.clone(),
        NonEmptyString::parse("scope:writer", "scope").unwrap(),
        None,
        None,
        None,
        None,
        None,
        Some(count(1)),
        None,
        StringMap::empty(),
    )
    .unwrap();
    let memory = IngestMemory::new(
        vec![IngestDimension::new(
            dimension,
            NonEmptyString::parse("agent", "kind").unwrap(),
            Some(NonEmptyString::parse("Writer", "title").unwrap()),
            StringMap::empty(),
        )],
        vec![
            IngestEntry::new(
                entry,
                NonEmptyString::parse("decision", "kind").unwrap(),
                NonEmptyString::parse("Calibrated write.", "text").unwrap(),
                vec![coordinate],
                StringMap::empty(),
            )
            .unwrap(),
        ],
        vec![],
        vec![],
    )
    .unwrap();
    tool(ToolArguments::Ingest(IngestArguments::new(
        subject.about().clone(),
        memory,
        None,
        NonEmptyString::parse("idem:calibration", "idempotency_key").unwrap(),
        true,
    )))
}

fn tool(arguments: ToolArguments) -> OperatorAction {
    OperatorAction::ToolCall(ToolCallAction::new(arguments))
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

fn second_ref(subject: &CalibrationSubject) -> MemoryRef {
    subject
        .visible_state()
        .known_refs()
        .iter()
        .nth(1)
        .cloned()
        .unwrap_or_else(|| first_ref(subject))
}

fn first_dim(subject: &CalibrationSubject) -> DimensionRef {
    subject
        .visible_state()
        .known_dimensions()
        .iter()
        .next()
        .cloned()
        .unwrap_or_else(|| DimensionRef::parse("agent:operator").unwrap())
}

fn count(value: usize) -> PositiveCount {
    PositiveCount::parse(value, "calibration").unwrap()
}

#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error("{0}")]
    Generic(String),
}
