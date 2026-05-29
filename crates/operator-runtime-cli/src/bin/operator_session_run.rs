use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use clap::{Parser, ValueEnum};
use operator_runtime_application::ports::mcp_executor_port::McpExecutor;
use operator_runtime_application::ports::operator_policy_port::OperatorPolicy;
use operator_runtime_application::use_cases::run_operator_session_single_step_use_case::RunOperatorSessionSingleStepUseCase;
use operator_runtime_domain::budget::session_budget::SessionBudget;
use operator_runtime_domain::session::observation::Observation;
use operator_runtime_domain::session::operator_request::OperatorRequest;
use operator_runtime_domain::session::operator_session_id::OperatorSessionId;
use operator_runtime_domain::session::outcome_class::OutcomeClass;
use operator_runtime_domain::session::session_outcome::SessionOutcome;
use operator_runtime_infra::adapters::anonymizing_operator_policy::AnonymizingOperatorPolicy;
use operator_runtime_infra::adapters::jsonl_session_event_sink::JsonlSessionEventSink;
use operator_runtime_infra::adapters::kmp_mcp_http_executor::KmpMcpHttpExecutor;
use operator_runtime_infra::adapters::kmp_mcp_stdio_config::KmpMcpStdioConfig;
use operator_runtime_infra::adapters::kmp_mcp_stdio_executor::KmpMcpStdioExecutor;
use operator_runtime_infra::adapters::vllm_openai_operator_policy::VllmOpenAiOperatorPolicy;
use operator_runtime_infra::adapters::vllm_operator_config::DEFAULT_MAX_TOKENS;
use operator_runtime_infra::adapters::vllm_operator_config::VllmOperatorConfig;
use operator_shared_domain::contract::composite_action_contract_validator::CompositeActionContractValidator;
use operator_shared_domain::tool::kernel_tool::KernelTool;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::visible_state::budget_field::BudgetField;
use operator_shared_infra::mappers::operator_action_mapper::OperatorActionMapper;
use operator_synthetic_infra::dto::calibration_subject_dto::CalibrationSubjectDto;
use operator_synthetic_infra::mappers::calibration_subject_mapper::CalibrationSubjectMapper;
use serde::Deserialize;
use serde_json::{Value, json};

const DEFAULT_ADAPTER_SHA: &str =
    "43186fa848c5f0e9d71915023f8f01c2341042de8aaf57b0c3c0c574a0f44379";

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum KmpMcpTransport {
    Stdio,
    Http,
}

#[derive(Debug, Parser)]
#[command(name = "operator-session-run")]
#[command(about = "Run single-step Operator runtime replay sessions")]
struct Cli {
    #[arg(long)]
    scenario_jsonl: PathBuf,
    #[arg(long)]
    output_dir: PathBuf,
    #[arg(long, default_value = "https://0.5b.llm.underpassai.com/v1")]
    operator_endpoint: String,
    #[arg(long, default_value = "operator-v8.1.2")]
    operator_adapter_id: String,
    #[arg(long)]
    operator_client_cert: Option<PathBuf>,
    #[arg(long)]
    operator_client_key: Option<PathBuf>,
    #[arg(long, default_value_t = false)]
    operator_accept_invalid_certs: bool,
    #[arg(long, default_value_t = DEFAULT_MAX_TOKENS)]
    operator_max_tokens: u32,
    /// Serve the anonymized operator (v8.1.9+): anonymize the subject's real refs
    /// to opaque ids before the model call and de-anonymize the predicted action
    /// back. Required when the served adapter was trained on anonymized refs.
    #[arg(long, default_value_t = false)]
    operator_anonymize_refs: bool,
    #[arg(long, default_value = DEFAULT_ADAPTER_SHA)]
    adapter_sha: String,
    #[arg(long)]
    #[arg(alias = "kmp-grpc-endpoint")]
    kmp_mcp_endpoint: String,
    #[arg(long, value_enum, default_value_t = KmpMcpTransport::Stdio)]
    kmp_mcp_transport: KmpMcpTransport,
    #[arg(long, default_value = "rehydration-mcp")]
    kmp_mcp_stdio_command: PathBuf,
    #[arg(long, default_value = "read")]
    mode: String,
    #[arg(long)]
    limit: Option<usize>,
    #[arg(long)]
    filter_tools: Option<String>,
}

fn main() {
    if let Err(err) = run(&Cli::parse()) {
        eprintln!("operator-session-run failed: {err}");
        std::process::exit(1);
    }
}

fn run(cli: &Cli) -> Result<(), String> {
    let mode_filter = ModeFilter::parse(cli.mode.as_str())?;
    let filter_tools = parse_filter_tools(cli.filter_tools.as_deref())?;
    let sessions_dir = cli.output_dir.join("sessions");
    let events_dir = cli.output_dir.join("events");
    fs::create_dir_all(&sessions_dir).map_err(|err| {
        format!(
            "create sessions output directory {}: {err}",
            sessions_dir.display()
        )
    })?;
    fs::create_dir_all(&events_dir).map_err(|err| {
        format!(
            "create events output directory {}: {err}",
            events_dir.display()
        )
    })?;

    let scenarios = load_scenarios(
        &cli.scenario_jsonl,
        mode_filter,
        filter_tools.as_ref(),
        cli.limit,
    )?;
    if scenarios.is_empty() {
        return Err("no scenarios selected after mode/tool filters".to_string());
    }
    let executor = build_mcp_executor(cli)?;

    let mut stats = ReplayStats::default();
    for scenario in scenarios {
        let base_policy = Arc::new(
            build_operator_policy(cli)?.with_system_prompt(scenario.system_prompt.clone()),
        ) as Arc<dyn OperatorPolicy>;
        let policy: Arc<dyn OperatorPolicy> = if cli.operator_anonymize_refs {
            Arc::new(AnonymizingOperatorPolicy::new(base_policy))
        } else {
            base_policy
        };
        let event_path = events_dir.join(format!("{}.jsonl", safe_filename(&scenario.step_id)));
        let event_sink = Arc::new(
            JsonlSessionEventSink::new(
                &event_path,
                cli.operator_endpoint.as_str(),
                cli.kmp_mcp_endpoint.as_str(),
                cli.adapter_sha.as_str(),
            )
            .map_err(|err| format!("open event sink {}: {err}", event_path.display()))?,
        );
        let use_case = RunOperatorSessionSingleStepUseCase::new(
            Arc::clone(&policy),
            Arc::clone(&executor),
            CompositeActionContractValidator::default_strict(),
            event_sink,
        );
        let outcome = use_case
            .execute(&scenario.request)
            .map_err(|err| format!("execute {}: {err}", scenario.step_id))?;
        stats.record(&outcome);
        write_outcome_row(&sessions_dir, &scenario, &outcome, cli)?;
        eprintln!(
            "session={} target={} predicted={} outcome={} elapsed_ms={}",
            scenario.step_id,
            scenario.target_label(),
            outcome.predicted_action().tool().map_or_else(
                || outcome.predicted_action().kind().as_str(),
                KernelTool::as_str
            ),
            outcome.outcome_class().as_str(),
            outcome.elapsed_ms()
        );
    }
    stats.print();
    write_summary(&cli.output_dir.join("summary.jsonl"), &stats)?;
    Ok(())
}

fn build_operator_policy(cli: &Cli) -> Result<VllmOpenAiOperatorPolicy, String> {
    let mut config = VllmOperatorConfig::new(
        cli.operator_endpoint.as_str(),
        cli.operator_adapter_id.as_str(),
    )
    .with_accept_invalid_certs(cli.operator_accept_invalid_certs)
    .with_max_tokens(cli.operator_max_tokens);
    match (&cli.operator_client_cert, &cli.operator_client_key) {
        (Some(cert), Some(key)) => {
            config = config.with_client_identity(cert, key);
        }
        (None, None) => {}
        (Some(_), None) | (None, Some(_)) => {
            return Err(
                "operator mTLS requires both --operator-client-cert and --operator-client-key"
                    .to_string(),
            );
        }
    }
    VllmOpenAiOperatorPolicy::new(&config).map_err(|err| err.to_string())
}

fn build_mcp_executor(cli: &Cli) -> Result<Arc<dyn McpExecutor>, String> {
    match cli.kmp_mcp_transport {
        KmpMcpTransport::Stdio => {
            let config = KmpMcpStdioConfig::new(
                cli.kmp_mcp_stdio_command.clone(),
                cli.kmp_mcp_endpoint.as_str(),
            );
            Ok(Arc::new(KmpMcpStdioExecutor::new(config)))
        }
        KmpMcpTransport::Http => KmpMcpHttpExecutor::new(cli.kmp_mcp_endpoint.as_str())
            .map(|executor| Arc::new(executor) as Arc<dyn McpExecutor>)
            .map_err(|err| err.to_string()),
    }
}

fn load_scenarios(
    path: &Path,
    mode_filter: ModeFilter,
    filter_tools: Option<&BTreeSet<String>>,
    limit: Option<usize>,
) -> Result<Vec<RuntimeScenario>, String> {
    let file = File::open(path).map_err(|err| format!("open {}: {err}", path.display()))?;
    let mut scenarios = Vec::new();
    for (line_index, line) in BufReader::new(file).lines().enumerate() {
        if limit.is_some_and(|max| scenarios.len() >= max) {
            break;
        }
        let line_number = line_index + 1;
        let json_line = line.map_err(|err| format!("read line {line_number}: {err}"))?;
        if json_line.trim().is_empty() {
            continue;
        }
        let eval_row: OpenAiEvalRow = serde_json::from_str(&json_line)
            .map_err(|err| format!("parse scenario JSON at line {line_number}: {err}"))?;
        let runtime_parts = eval_row.into_runtime_parts(line_number)?;
        let subject = runtime_parts.subject(line_number)?;
        let target = runtime_parts.target_action();
        if !mode_filter.accepts(subject.mode.as_str(), &target) {
            continue;
        }
        if let Some(filter) = filter_tools
            && !target
                .tool
                .as_ref()
                .is_some_and(|tool| filter.contains(tool))
        {
            continue;
        }
        scenarios.push(RuntimeScenario::from_row(runtime_parts, &subject, target)?);
    }
    Ok(scenarios)
}

#[derive(Debug, Clone, Copy)]
enum ModeFilter {
    Exact(&'static str),
    ReadProfile,
}

impl ModeFilter {
    fn parse(raw: &str) -> Result<Self, String> {
        match raw {
            "read-profile" | "read_profile" => Ok(Self::ReadProfile),
            "read" => Ok(Self::Exact("read")),
            "write" => Ok(Self::Exact("write")),
            "full" => Ok(Self::Exact("full")),
            "writer_pre_read" | "writer-pre-read" => Ok(Self::Exact("writer_pre_read")),
            other => Err(format!(
                "unsupported mode '{other}'; expected read, write, full, writer_pre_read, or read_profile"
            )),
        }
    }

    fn accepts(self, scenario_mode: &str, target: &TargetActionSummary) -> bool {
        match self {
            Self::Exact(expected) => scenario_mode == expected,
            Self::ReadProfile => {
                matches!(scenario_mode, "read" | "writer_pre_read")
                    && !matches!(
                        target.tool.as_deref(),
                        Some("kernel_ingest" | "kernel_write_memory")
                    )
            }
        }
    }
}

fn parse_filter_tools(raw: Option<&str>) -> Result<Option<BTreeSet<String>>, String> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let mut tools = BTreeSet::new();
    for item in raw
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        KernelTool::parse(item).map_err(|err| err.to_string())?;
        tools.insert(item.to_string());
    }
    Ok(Some(tools))
}

#[derive(Debug)]
struct RuntimeScenario {
    step_id: String,
    system_prompt: String,
    request: OperatorRequest,
    target: TargetActionSummary,
}

impl RuntimeScenario {
    fn from_row(
        row: OpenAiEvalRow,
        subject_dto: &CalibrationSubjectDto,
        target: TargetActionSummary,
    ) -> Result<Self, String> {
        let subject = CalibrationSubjectMapper::to_domain(subject_dto)
            .map_err(|err| format!("map subject for {}: {err}", row.step_id))?;
        let budget = session_budget_from_visible(subject.visible_state())?;
        // Replay-side pass-through: if the SFT scenario carried a top-level
        // prepared_action (as v8.1.5 corpus rows do for kernel_ask), preserve
        // it on the OperatorRequest so the runtime can re-inject it into the
        // subject sent to vLLM. Validation against allowed_tools happens later
        // inside CalibrationSubject::new.
        let prepared_action = subject
            .prepared_action()
            .map(|prepared| prepared.action().clone());
        let request = OperatorRequest::new(
            OperatorSessionId::parse(row.step_id.clone()).map_err(|err| err.to_string())?,
            subject.goal().clone(),
            subject.visible_state().clone(),
            subject.mode(),
            subject.allowed_tools().clone(),
            budget,
            subject.about().clone(),
            prepared_action,
        )
        .map_err(|err| format!("build operator request for {}: {err}", row.step_id))?;
        Ok(Self {
            step_id: row.step_id,
            system_prompt: row.system_prompt,
            request,
            target,
        })
    }

    fn target_label(&self) -> &str {
        self.target
            .tool
            .as_deref()
            .unwrap_or(self.target.kind.as_str())
    }
}

fn session_budget_from_visible(
    visible: &operator_shared_domain::visible_state::visible_state::VisibleState,
) -> Result<SessionBudget, String> {
    Ok(SessionBudget::new(
        budget_field_to_u32(visible.budget().calls_remaining(), "calls_remaining")?,
        budget_field_to_u32(visible.budget().tokens_remaining(), "tokens_remaining")?,
    ))
}

fn budget_field_to_u32(field: BudgetField, field_name: &str) -> Result<u32, String> {
    match field {
        BudgetField::Unbounded => Ok(u32::MAX),
        BudgetField::Bounded(value) => u32::try_from(value).map_err(|_| {
            format!(
                "visible_state.budget.{field_name} bounded value {value} exceeds runtime u32 limit"
            )
        }),
    }
}

#[derive(Debug, Deserialize)]
struct OpenAiEvalRow {
    step_id: String,
    #[serde(default)]
    system_prompt: String,
    messages: Vec<OpenAiMessage>,
}

impl OpenAiEvalRow {
    fn subject(&self, line_number: usize) -> Result<CalibrationSubjectDto, String> {
        let content = self
            .messages
            .iter()
            .find(|message| message.role == "user")
            .ok_or_else(|| format!("line {line_number} has no user message"))?
            .content
            .as_str();
        serde_json::from_str(content).map_err(|err| {
            format!("line {line_number} user content is not CalibrationSubjectDto JSON: {err}")
        })
    }

    fn into_runtime_parts(mut self, line_number: usize) -> Result<Self, String> {
        let system_prompt = self
            .messages
            .iter()
            .find(|message| message.role == "system")
            .ok_or_else(|| format!("line {line_number} has no system message"))?
            .content
            .clone();
        self.system_prompt = system_prompt;
        Ok(self)
    }

    fn target_action(&self) -> TargetActionSummary {
        self.messages
            .iter()
            .find(|message| message.role == "assistant")
            .and_then(|message| serde_json::from_str::<Value>(&message.content).ok())
            .map_or_else(TargetActionSummary::unknown, |value| {
                TargetActionSummary::from_action_value(&value)
            })
    }
}

#[derive(Debug, Deserialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TargetActionSummary {
    kind: String,
    tool: Option<String>,
}

impl TargetActionSummary {
    fn unknown() -> Self {
        Self {
            kind: "unknown".to_string(),
            tool: None,
        }
    }

    fn from_action_value(value: &Value) -> Self {
        let action = value.get("action").unwrap_or(value);
        Self {
            kind: action
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            tool: action
                .get("tool")
                .and_then(Value::as_str)
                .map(str::to_string),
        }
    }
}

#[derive(Debug, Default)]
struct ReplayStats {
    total: u32,
    outcome_counts: BTreeMap<&'static str, u32>,
    mcp_attempted: u32,
    mcp_completed: u32,
}

impl ReplayStats {
    fn record(&mut self, outcome: &SessionOutcome) {
        self.total += 1;
        *self
            .outcome_counts
            .entry(outcome.outcome_class().as_str())
            .or_default() += 1;
        if outcome.predicted_action().tool().is_some() {
            self.mcp_attempted += 1;
            if matches!(outcome.outcome_class(), OutcomeClass::Completed) {
                self.mcp_completed += 1;
            }
        }
    }

    fn mcp_execution_success_rate(&self) -> Option<f64> {
        if self.mcp_attempted == 0 {
            None
        } else {
            Some(f64::from(self.mcp_completed) / f64::from(self.mcp_attempted))
        }
    }

    fn print(&self) {
        eprintln!("total_sessions: {}", self.total);
        for (class, count) in &self.outcome_counts {
            eprintln!("outcome.{class}: {count}");
        }
        eprintln!("mcp_attempted: {}", self.mcp_attempted);
        eprintln!("mcp_completed: {}", self.mcp_completed);
        match self.mcp_execution_success_rate() {
            Some(rate) => eprintln!("mcp_execution_success_rate: {rate:.4}"),
            None => eprintln!("mcp_execution_success_rate: n/a"),
        }
    }
}

fn write_outcome_row(
    sessions_dir: &Path,
    scenario: &RuntimeScenario,
    outcome: &SessionOutcome,
    cli: &Cli,
) -> Result<(), String> {
    let path = sessions_dir.join(format!("{}.jsonl", safe_filename(&scenario.step_id)));
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|err| format!("open session outcome {}: {err}", path.display()))?;
    let row = outcome_row(scenario, outcome, cli)?;
    writeln!(file, "{row}")
        .map_err(|err| format!("write session outcome {}: {err}", path.display()))
}

fn write_summary(path: &Path, stats: &ReplayStats) -> Result<(), String> {
    let value = json!({
        "total_sessions": stats.total,
        "outcome_counts": stats.outcome_counts,
        "mcp_attempted": stats.mcp_attempted,
        "mcp_completed": stats.mcp_completed,
        "mcp_execution_success_rate": stats.mcp_execution_success_rate(),
    });
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("open summary {}: {err}", path.display()))?;
    writeln!(file, "{value}").map_err(|err| format!("write summary {}: {err}", path.display()))
}

fn outcome_row(
    scenario: &RuntimeScenario,
    outcome: &SessionOutcome,
    cli: &Cli,
) -> Result<Value, String> {
    let predicted_action =
        OperatorActionMapper::to_dto(outcome.predicted_action()).map_err(|err| err.to_string())?;
    Ok(json!({
        "session_id": outcome.session_id().as_str(),
        "step_id": scenario.step_id.as_str(),
        "target_action": {
            "kind": scenario.target.kind.as_str(),
            "tool": scenario.target.tool.as_deref(),
        },
        "predicted_action": {
            "kind": outcome.predicted_action().kind().as_str(),
            "tool": outcome.predicted_action().tool().map(KernelTool::as_str),
            "wire": predicted_action,
        },
        "observation": observation_json(outcome.observation()),
        "outcome": {
            "class": outcome.outcome_class().as_str(),
            "elapsed_ms": outcome.elapsed_ms(),
            "final_budget": {
                "calls_remaining": outcome.final_budget().calls_remaining(),
                "tokens_remaining": outcome.final_budget().tokens_remaining(),
            },
            "violations": violations_json(outcome.outcome_class()),
        },
        "adapter_sha": cli.adapter_sha.as_str(),
        "operator_endpoint": cli.operator_endpoint.as_str(),
        "mcp_endpoint": cli.kmp_mcp_endpoint.as_str(),
        "mcp_transport": format!("{:?}", cli.kmp_mcp_transport).to_lowercase(),
    }))
}

fn observation_json(observation: Option<&Observation>) -> Value {
    match observation {
        Some(Observation::ToolResponse {
            outcome,
            observed_refs,
        }) => json!({
            "class": "tool_response",
            "tool": outcome.tool().as_str(),
            "observed_refs": observed_refs.iter().map(MemoryRef::as_str).collect::<Vec<_>>(),
        }),
        Some(Observation::ToolError { code, message }) => json!({
            "class": "tool_error",
            "code": code.as_str(),
            "message": message,
        }),
        Some(Observation::Terminal { reason }) => json!({
            "class": "terminal",
            "reason": reason.as_str(),
        }),
        None => Value::Null,
    }
}

fn violations_json(outcome_class: &OutcomeClass) -> Value {
    match outcome_class {
        OutcomeClass::ContractViolation { violations } => Value::Array(
            violations
                .as_slice()
                .iter()
                .map(|violation| {
                    json!({
                        "code": violation.code().as_str(),
                        "field": violation.field(),
                        "message": violation.message(),
                    })
                })
                .collect(),
        ),
        _ => Value::Array(Vec::new()),
    }
}

fn safe_filename(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::action::operator_action::OperatorAction;
    use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;

    #[test]
    fn budget_field_keeps_unbounded_distinct_from_huge_bounded_values() {
        assert_eq!(
            budget_field_to_u32(BudgetField::Unbounded, "calls_remaining").unwrap(),
            u32::MAX
        );

        let too_large =
            usize::try_from(u64::from(u32::MAX) + 1).expect("test platform fits u32::MAX + 1");
        let error = budget_field_to_u32(BudgetField::Bounded(too_large), "calls_remaining")
            .expect_err("huge bounded budget must not saturate");

        assert!(error.contains("exceeds runtime u32 limit"));
    }

    fn scenario_subject_dto_with_prepared_action() -> CalibrationSubjectDto {
        // Minimal valid CalibrationSubjectDto carrying a kernel_ask
        // prepared_action. Mirrors the shape v8.1.5 corpus rows use after the
        // Python data-prep step injects PA.
        let raw = r#"{
            "about": "about:replay:test:case-0001",
            "mode": "read",
            "task_family": "realistic.kernel_ask.replay",
            "goal": "Bounded probe for replay PA preservation.",
            "allowed_tools": [
                "kernel_wake",
                "kernel_ask",
                "kernel_near",
                "kernel_goto",
                "kernel_rewind",
                "kernel_forward",
                "kernel_trace",
                "kernel_inspect"
            ],
            "visible_state": {
                "known_refs": [],
                "known_dimensions": [],
                "budget": {"calls_remaining": 1, "tokens_remaining": 4096}
            },
            "prepared_action": {
                "kind": "tool_call",
                "tool": "kernel_ask",
                "arguments": {"query": "What is the active constraint?"}
            }
        }"#;
        serde_json::from_str(raw).expect("static scenario DTO parses")
    }

    fn target_summary_for_ask(query: &str) -> TargetActionSummary {
        let value = serde_json::json!({
            "action": {
                "kind": "tool_call",
                "tool": "kernel_ask",
                "arguments": {"query": query}
            }
        });
        TargetActionSummary::from_action_value(&value)
    }

    #[test]
    fn replay_propagates_prepared_action_from_subject_into_request() {
        // Mirrors what `load_scenarios` does for one row whose user content
        // already carries a top-level prepared_action (v8.1.5 corpus shape).
        // This is the only place where the bin reads PA off the wire and
        // hands it to the runtime; if this regresses, the v8.1.5 mechanism
        // silently degrades to v8.1.3pa-without-PA at replay time.
        let dto = scenario_subject_dto_with_prepared_action();
        let row = OpenAiEvalRow {
            step_id: "scenario:kernel_ask:replay:0001:step:0001".to_string(),
            system_prompt: "system".to_string(),
            messages: vec![],
        };
        let target = target_summary_for_ask("What is the active constraint?");

        let scenario = RuntimeScenario::from_row(row, &dto, target)
            .expect("scenario builds when PA is well-formed");

        let pa = scenario
            .request
            .prepared_action()
            .expect("OperatorRequest carries prepared_action from the scenario DTO");
        match pa {
            OperatorAction::ToolCall(call) => match call.arguments() {
                ToolArguments::Ask(ask) => {
                    assert_eq!(ask.query().as_str(), "What is the active constraint?");
                }
                other => panic!("expected ask arguments, got {other:?}"),
            },
            other => panic!("expected tool_call action, got {other:?}"),
        }
    }

    #[test]
    fn replay_preserves_none_when_scenario_has_no_prepared_action() {
        let dto: CalibrationSubjectDto = serde_json::from_str(
            r#"{
                "about": "about:replay:no-pa:case-0001",
                "mode": "read",
                "task_family": "realistic.kernel_inspect.replay",
                "goal": "Bounded inspect for replay-no-PA preservation.",
                "allowed_tools": [
                    "kernel_wake",
                    "kernel_ask",
                    "kernel_near",
                    "kernel_goto",
                    "kernel_rewind",
                    "kernel_forward",
                    "kernel_trace",
                    "kernel_inspect"
                ],
                "visible_state": {
                    "known_refs": [],
                    "known_dimensions": [],
                    "budget": {"calls_remaining": 1, "tokens_remaining": 4096}
                }
            }"#,
        )
        .expect("static scenario DTO without PA parses");
        let row = OpenAiEvalRow {
            step_id: "scenario:kernel_inspect:replay:0001:step:0001".to_string(),
            system_prompt: "system".to_string(),
            messages: vec![],
        };
        let target = target_summary_for_ask("ignored");
        let scenario =
            RuntimeScenario::from_row(row, &dto, target).expect("scenario builds without PA");
        assert!(scenario.request.prepared_action().is_none());
    }
}
