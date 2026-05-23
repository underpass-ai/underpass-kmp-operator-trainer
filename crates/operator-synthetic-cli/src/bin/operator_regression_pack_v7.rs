//! `operator-regression-pack-v7` — replay diagnosed v7.3 scenarios by id.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;
use operator_shared_domain::action::escalate_action::EscalateAction;
use operator_shared_domain::action::escalate_reason::EscalateReason;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::action::stop_action::StopAction;
use operator_shared_domain::action::stop_reason::StopReason;
use operator_shared_domain::action::tool_call_action::ToolCallAction;
use operator_shared_domain::cursor::cursor::Cursor;
use operator_shared_domain::cursor::ref_cursor::RefCursor;
use operator_shared_domain::cursor::temporal_anchor::TemporalAnchor;
use operator_shared_domain::cursor::temporal_cursor::TemporalCursor;
use operator_shared_domain::cursor::temporal_cursor_key::TemporalCursorKey;
use operator_shared_domain::cursor::trace_cursor::TraceCursor;
use operator_shared_domain::tool_arguments::goto_arguments::GotoArguments;
use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
use operator_shared_domain::tool_arguments::wake_arguments::WakeArguments;
use operator_shared_domain::value_objects::finish_reason::FinishReason;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::model_id::ModelId;
use operator_shared_domain::value_objects::subject_hash::SubjectHash;
use operator_synthetic_application::error::scenario_source_error::ScenarioSourceError;
use operator_synthetic_application::error::teacher_policy_error::TeacherPolicyError;
use operator_synthetic_application::ports::scenario::Scenario;
use operator_synthetic_application::ports::scenario_source::ScenarioSource;
use operator_synthetic_application::ports::teacher_policy::TeacherPolicy;
use operator_synthetic_application::use_cases::build_realistic_corpus_use_case::BuildRealisticCorpusUseCase;
use operator_synthetic_application::use_cases::max_drop_rate::MaxDropRate;
use operator_synthetic_application::use_cases::realistic_corpus_report::RealisticCorpusReport;
use operator_synthetic_domain::calibration::calibration_subject::CalibrationSubject;
use operator_synthetic_domain::calibration::teacher_decision::TeacherDecision;
use operator_synthetic_infra::adapters::composite_corpus_event_sink::CompositeCorpusEventSink;
use operator_synthetic_infra::adapters::jsonl_scenario_source::JsonlScenarioSource;
use operator_synthetic_infra::adapters::jsonl_streaming_sink::{
    DROPPED_PARTIAL_FILE, JsonlStreamingSink, TRAJECTORIES_PARTIAL_FILE,
};
use operator_synthetic_infra::adapters::openai_compatible_teacher_policy::OpenAiCompatibleTeacherPolicy;
use operator_synthetic_infra::adapters::stderr_progress_sink::StderrProgressSink;
use reqwest::Url;

#[derive(Parser, Debug)]
#[command(
    name = "operator-regression-pack-v7",
    about = "Replay diagnosed v7.3 scenarios through the realistic corpus use case.",
    version
)]
struct Cli {
    #[arg(long)]
    scenarios: PathBuf,
    #[arg(long)]
    pack: PathBuf,
    #[arg(long)]
    output: PathBuf,
    #[arg(long, default_value = "https://api.openai.com/v1")]
    api_base: String,
    #[arg(long)]
    api_key_file: Option<PathBuf>,
    #[arg(long)]
    prompt: Option<PathBuf>,
    #[arg(long, default_value = "gpt-4o-mini")]
    model: String,
    #[arg(long, default_value_t = 0.0)]
    temperature: f32,
    #[arg(long)]
    mock_teacher: bool,
    #[arg(long)]
    mock_wrong: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(err) => {
            eprintln!("regression-pack failed: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<bool, CliError> {
    precheck(cli)?;
    let scenario_ids = read_pack(&cli.pack)?;
    let scenarios = load_selected_scenarios(&cli.scenarios, &scenario_ids)?;
    fs::create_dir_all(&cli.output).map_err(|err| {
        CliError::Generic(format!("create --output {}: {err}", cli.output.display()))
    })?;
    let source = InMemoryScenarioSource::new(scenarios);
    let teacher = teacher_policy(cli)?;
    let validator =
        operator_shared_domain::contract::composite_action_contract_validator::CompositeActionContractValidator::default_strict();
    let event_sink = CompositeCorpusEventSink::new(vec![
        Box::new(StderrProgressSink::default()),
        Box::new(
            JsonlStreamingSink::new(&cli.output)
                .map_err(|err| CliError::Generic(format!("build event sink: {err}")))?,
        ),
    ]);
    let use_case = BuildRealisticCorpusUseCase::new(source, teacher, validator, event_sink);
    let report = use_case
        .execute(
            MaxDropRate::parse(0.0).map_err(|err| CliError::Generic(err.to_string()))?,
            None,
        )
        .map_err(|err| CliError::Generic(format!("run regression pack: {err}")))?;
    promote_streamed_outputs(&cli.output)?;
    print_results(&report);
    Ok(report.gate_passed())
}

fn precheck(cli: &Cli) -> Result<(), CliError> {
    require_readable_non_empty(&cli.scenarios, "--scenarios")?;
    require_readable_non_empty(&cli.pack, "--pack")?;
    if cli.mock_teacher || cli.mock_wrong {
        return Ok(());
    }
    let prompt = cli.prompt.as_ref().ok_or_else(|| {
        CliError::Generic("--prompt is required without --mock-teacher".to_string())
    })?;
    let api_key_file = cli.api_key_file.as_ref().ok_or_else(|| {
        CliError::Generic("--api-key-file is required without --mock-teacher".to_string())
    })?;
    require_readable_non_empty(prompt, "--prompt")?;
    require_readable_non_empty(api_key_file, "--api-key-file")?;
    if read_key(api_key_file)?.is_empty() {
        return Err(CliError::Generic(
            "--api-key-file is empty after trim".to_string(),
        ));
    }
    Url::parse(&cli.api_base)
        .map_err(|err| CliError::Generic(format!("invalid --api-base: {err}")))?;
    Ok(())
}

fn teacher_policy(cli: &Cli) -> Result<Box<dyn TeacherPolicy>, CliError> {
    if cli.mock_wrong {
        return Ok(Box::new(MockTeacherPolicy::Wrong));
    }
    if cli.mock_teacher {
        return Ok(Box::new(MockTeacherPolicy::Accepted));
    }
    let prompt = cli.prompt.as_ref().ok_or_else(|| {
        CliError::Generic("--prompt is required without --mock-teacher".to_string())
    })?;
    let api_key_file = cli.api_key_file.as_ref().ok_or_else(|| {
        CliError::Generic("--api-key-file is required without --mock-teacher".to_string())
    })?;
    Ok(Box::new(
        OpenAiCompatibleTeacherPolicy::new(
            &cli.api_base,
            Some(read_key(api_key_file)?),
            &cli.model,
            prompt,
        )
        .map_err(|err| CliError::Generic(format!("build teacher adapter: {err}")))?
        .with_temperature(cli.temperature),
    ))
}

fn read_pack(path: &Path) -> Result<Vec<String>, CliError> {
    let mut ids = Vec::new();
    for (zero_based, line) in fs::read_to_string(path)
        .map_err(|err| CliError::Generic(format!("read --pack {}: {err}", path.display())))?
        .lines()
        .enumerate()
    {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if !line.starts_with("scenario:") {
            return Err(CliError::Generic(format!(
                "{}:{}: expected scenario id, got '{line}'",
                path.display(),
                zero_based + 1
            )));
        }
        ids.push(line.to_string());
    }
    if ids.is_empty() {
        return Err(CliError::Generic(format!(
            "--pack {} did not contain scenario ids",
            path.display()
        )));
    }
    Ok(ids)
}

fn load_selected_scenarios(path: &Path, ids: &[String]) -> Result<Vec<Scenario>, CliError> {
    let all = JsonlScenarioSource::new(path)
        .read()
        .map_err(|err| CliError::Generic(format!("read scenarios: {err}")))?;
    let by_id: BTreeMap<String, Scenario> = all
        .into_iter()
        .map(|scenario| (scenario.id().as_str().to_string(), scenario))
        .collect();
    let mut out = Vec::with_capacity(ids.len());
    let mut missing = Vec::new();
    let mut seen = BTreeSet::new();
    for id in ids {
        if !seen.insert(id.clone()) {
            return Err(CliError::Generic(format!(
                "duplicate scenario id in pack: {id}"
            )));
        }
        match by_id.get(id) {
            Some(scenario) => out.push(scenario.clone()),
            None => missing.push(id.clone()),
        }
    }
    if !missing.is_empty() {
        return Err(CliError::Generic(format!(
            "pack scenarios not found in {}: {}",
            path.display(),
            missing.join(", ")
        )));
    }
    Ok(out)
}

fn promote_streamed_outputs(output: &Path) -> Result<(), CliError> {
    promote_file(
        &output.join(TRAJECTORIES_PARTIAL_FILE),
        &output.join("trajectories.jsonl"),
    )?;
    promote_file(
        &output.join(DROPPED_PARTIAL_FILE),
        &output.join("dropped.jsonl"),
    )
}

fn promote_file(from: &Path, to: &Path) -> Result<(), CliError> {
    fs::rename(from, to).map_err(|err| {
        CliError::Generic(format!(
            "promote {} to {}: {err}",
            from.display(),
            to.display()
        ))
    })
}

fn print_results(report: &RealisticCorpusReport) {
    println!("scenario_id\ttarget_match\tsemantic_match\tpredicted_action");
    for row in report.accepted() {
        let step_id = row.step_id().as_str();
        let scenario_id = step_id.split(":step:").next().unwrap_or(step_id);
        println!(
            "{}\ttrue\ttrue\t{}",
            scenario_id,
            action_label(row.target_action())
        );
    }
    for drop in report.dropped() {
        let (target_match, semantic_match) = match drop.reason().kind().as_str() {
            "semantic_mismatch" => ("true", "false"),
            _ => ("false", "not_evaluated"),
        };
        let action = drop
            .predicted_action()
            .map_or_else(|| "none".to_string(), action_label);
        println!(
            "{}\t{}\t{}\t{}",
            drop.scenario_id().as_str(),
            target_match,
            semantic_match,
            action
        );
    }
    println!("accepted_count: {}", report.accepted_count());
    println!("dropped_count: {}", report.dropped_count());
    println!("gate_passed: {}", report.gate_passed());
}

fn action_label(action: &OperatorAction) -> String {
    action.tool().map_or_else(
        || action.kind().as_str().to_string(),
        |tool| tool.as_str().to_string(),
    )
}

#[derive(Debug, Clone)]
struct InMemoryScenarioSource {
    scenarios: Vec<Scenario>,
}

impl InMemoryScenarioSource {
    fn new(scenarios: Vec<Scenario>) -> Self {
        Self { scenarios }
    }
}

impl ScenarioSource for InMemoryScenarioSource {
    fn read(&self) -> Result<Vec<Scenario>, ScenarioSourceError> {
        Ok(self.scenarios.clone())
    }
}

#[derive(Debug)]
enum MockTeacherPolicy {
    Accepted,
    Wrong,
}

impl TeacherPolicy for MockTeacherPolicy {
    fn decide(&self, subject: &CalibrationSubject) -> Result<TeacherDecision, TeacherPolicyError> {
        match self {
            Self::Accepted => Ok(mock_decision(mock_action_for_subject(subject))),
            Self::Wrong => Ok(mock_decision(OperatorAction::ToolCall(
                ToolCallAction::new(ToolArguments::Wake(WakeArguments::new(
                    subject.about().clone(),
                ))),
            ))),
        }
    }
}

fn mock_decision(action: OperatorAction) -> TeacherDecision {
    TeacherDecision::new(action, FinishReason::Stop, mock_subject_hash())
}

fn mock_subject_hash() -> SubjectHash {
    SubjectHash::parse("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
        .expect("mock subject hash is valid")
}

fn mock_action_for_subject(subject: &CalibrationSubject) -> OperatorAction {
    if let Some(prepared) = subject.prepared_action() {
        return prepared.action().clone();
    }
    let family = subject.task_family().as_str();
    if family.contains("kernel_goto") {
        return goto_action_for_family(subject, family);
    }
    if family.contains("kernel_inspect") {
        return OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
            InspectArguments::new(first_ref(subject)),
        )));
    }
    if family.contains("stop") {
        return stop_action_for_family(subject, family);
    }
    if family.contains("escalate") {
        return OperatorAction::Escalate(EscalateAction::new(
            EscalateReason::BeyondCapability,
            ModelId::parse("frontier-reasoner").expect("model id parses"),
        ));
    }
    OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Wake(
        WakeArguments::new(subject.about().clone()),
    )))
}

fn goto_action_for_family(subject: &CalibrationSubject, family: &str) -> OperatorAction {
    let cursor = if family.contains("temporal-cursor") {
        Cursor::Temporal(TemporalCursor::new(
            TemporalCursorKey::Updated,
            TemporalAnchor::parse("2026-05-23T00:00:00Z").expect("temporal anchor parses"),
        ))
    } else if family.contains("trace-cursor") {
        let (from, to) = first_two_refs(subject);
        Cursor::Trace(TraceCursor::new(from, to))
    } else {
        Cursor::Ref(RefCursor::new(first_ref(subject)))
    };
    OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Goto(
        GotoArguments::new(cursor),
    )))
}

fn stop_action_for_family(subject: &CalibrationSubject, family: &str) -> OperatorAction {
    let reason = if family.contains("no-candidate") {
        StopReason::NoCandidate
    } else if family.contains("budget-exhausted") {
        StopReason::BudgetExhausted
    } else {
        StopReason::AnswerReady
    };
    let evidence = match reason {
        StopReason::AnswerReady => vec![first_ref(subject)],
        StopReason::NoCandidate | StopReason::BudgetExhausted => vec![],
    };
    OperatorAction::Stop(
        StopAction::new(
            reason,
            Some("Regression pack mock answer.".to_string()),
            evidence,
        )
        .expect("mock stop action builds"),
    )
}

fn first_ref(subject: &CalibrationSubject) -> MemoryRef {
    subject
        .visible_state()
        .known_refs()
        .iter()
        .next()
        .cloned()
        .unwrap_or_else(|| MemoryRef::parse("node:visible").expect("fallback ref parses"))
}

fn first_two_refs(subject: &CalibrationSubject) -> (MemoryRef, MemoryRef) {
    let mut refs = subject.visible_state().known_refs().iter().cloned();
    let first = refs
        .next()
        .unwrap_or_else(|| MemoryRef::parse("node:trace-from").expect("fallback ref parses"));
    let second = refs.next().unwrap_or_else(|| first.clone());
    (first, second)
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

#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error("{0}")]
    Generic(String),
}
