//! Free-run the TRAINED operator (served via vLLM) through window-expansion
//! sessions against a live KMP endpoint and emit the operand set it surfaces.
//!
//! Unlike the `operator_generate_*` bins (teacher policy → SFT corpus), the
//! policy here is the STUDENT itself ([`VllmOpenAiOperatorPolicy`], optionally
//! anonymized to match the trained subject format). So this validates the
//! deployed operator end-to-end — it autonomously wakes a bounded window and
//! `kernel_near`-expands until it stops — and writes the operand set
//! (`final_visible_state.known_refs`) a downstream reader (e.g. gpt-4o) derives
//! the answer over. The operator owns operand-set COMPLETENESS; derivation/count
//! is the reader's job and is intentionally out of scope here.

use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, ValueEnum};
use operator_runtime_application::ports::mcp_executor_port::McpExecutor;
use operator_runtime_application::ports::operator_policy_port::OperatorPolicy;
use operator_runtime_application::services::window_expansion_episode_compiler::WindowExpansionEpisodeCompiler;
use operator_runtime_application::use_cases::run_operator_session_multi_step_use_case::RunOperatorSessionMultiStepUseCase;
use operator_runtime_domain::session::operator_session_id::OperatorSessionId;
use operator_runtime_infra::adapters::anonymizing_operator_policy::AnonymizingOperatorPolicy;
use operator_runtime_infra::adapters::kmp_mcp_http_executor::KmpMcpHttpExecutor;
use operator_runtime_infra::adapters::kmp_mcp_stdio_config::KmpMcpStdioConfig;
use operator_runtime_infra::adapters::kmp_mcp_stdio_executor::KmpMcpStdioExecutor;
use operator_runtime_infra::adapters::stderr_session_event_sink::StderrSessionEventSink;
use operator_runtime_infra::adapters::vllm_openai_operator_policy::VllmOpenAiOperatorPolicy;
use operator_runtime_infra::adapters::vllm_operator_config::VllmOperatorConfig;
use operator_shared_domain::contract::composite_action_contract_validator::CompositeActionContractValidator;
use operator_synthetic_infra::adapters::jsonl_window_expansion_episode_source::JsonlWindowExpansionEpisodeSource;
use serde_json::json;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum KmpMcpTransport {
    Stdio,
    Http,
}

#[derive(Debug, Parser)]
#[command(name = "operator-window-freerun")]
#[command(about = "Free-run the trained operator through window-expansion sessions")]
struct Cli {
    #[arg(long)]
    episodes_jsonl: PathBuf,
    #[arg(long)]
    output_path: PathBuf,
    #[arg(long, default_value = "https://0.5b.llm.underpassai.com/v1")]
    operator_endpoint: String,
    #[arg(long, default_value = "winexp")]
    operator_adapter_id: String,
    #[arg(long, default_value_t = 512)]
    operator_max_tokens: u32,
    #[arg(long, default_value_t = false)]
    operator_accept_invalid_certs: bool,
    /// Anonymize the subject's real refs to opaque ids before the model call and
    /// de-anonymize the predicted action back — required when the served adapter
    /// was trained on anonymized refs (the about-anonymized window corpus is).
    #[arg(long, default_value_t = false)]
    operator_anonymize_refs: bool,
    /// Force the model-facing subject `task_family` and synthesize
    /// `operator_state` so the served subject matches the SFT training format
    /// (the trained adapter's `task_family`, e.g. `runtime.window_expansion`).
    #[arg(long, default_value = "runtime.window_expansion")]
    operator_task_family: String,
    #[arg(long, alias = "kmp-grpc-endpoint")]
    kmp_mcp_endpoint: String,
    #[arg(long, value_enum, default_value_t = KmpMcpTransport::Stdio)]
    kmp_mcp_transport: KmpMcpTransport,
    #[arg(long, default_value = "rehydration-mcp")]
    kmp_mcp_stdio_command: PathBuf,
    #[arg(long)]
    limit: Option<usize>,
}

fn main() {
    if let Err(err) = run(&Cli::parse()) {
        eprintln!("operator-window-freerun failed: {err}");
        std::process::exit(1);
    }
}

fn run(cli: &Cli) -> Result<(), String> {
    let mut source = JsonlWindowExpansionEpisodeSource::new(&cli.episodes_jsonl);
    if let Some(limit) = cli.limit {
        source = source.with_limit(limit);
    }
    let episodes = source.read().map_err(|err| err.to_string())?;

    let config = VllmOperatorConfig::new(cli.operator_endpoint.as_str(), cli.operator_adapter_id.as_str())
        .with_accept_invalid_certs(cli.operator_accept_invalid_certs)
        .with_max_tokens(cli.operator_max_tokens);
    let base_policy: Arc<dyn OperatorPolicy> = Arc::new(
        VllmOpenAiOperatorPolicy::new(&config)
            .map_err(|err| err.to_string())?
            .with_model_facing_task_family(cli.operator_task_family.as_str()),
    );
    let policy: Arc<dyn OperatorPolicy> = if cli.operator_anonymize_refs {
        Arc::new(AnonymizingOperatorPolicy::new(base_policy))
    } else {
        base_policy
    };

    let session = RunOperatorSessionMultiStepUseCase::new(
        policy,
        build_mcp_executor(cli)?,
        CompositeActionContractValidator::default_strict(),
        Arc::new(StderrSessionEventSink::new()),
    );

    let mut records = Vec::with_capacity(episodes.len());
    for (index, episode) in episodes.iter().enumerate() {
        let session_id = OperatorSessionId::parse(format!("freerun:{index:04}"))
            .map_err(|err| err.to_string())?;
        let request = WindowExpansionEpisodeCompiler::compile(
            session_id,
            episode.about().clone(),
            episode.goal().clone(),
            &episode.spec(),
            episode.token_budget(),
        )
        .map_err(|err| err.to_string())?;
        let transcript = session.execute(&request).map_err(|err| err.to_string())?;
        let final_state = transcript.final_visible_state();
        let operand: Vec<&str> = final_state
            .known_refs()
            .iter()
            .map(operator_shared_domain::value_objects::memory_ref::MemoryRef::as_str)
            .collect();
        let expected: Vec<&str> = episode
            .expected_refs()
            .iter()
            .map(operator_shared_domain::value_objects::memory_ref::MemoryRef::as_str)
            .collect();
        let covered = expected.iter().all(|r| operand.contains(r));
        eprintln!(
            "freerun about={} steps={} operand={} expected={} covered={}",
            episode.about().as_str(),
            transcript.step_count(),
            operand.len(),
            expected.len(),
            covered,
        );
        records.push(json!({
            "about": episode.about().as_str(),
            "goal": episode.goal().as_str(),
            "step_count": transcript.step_count(),
            "terminal_action_kind": terminal_kind(&transcript),
            "operand_refs": operand,
            "expected_refs": expected,
            "covered": covered,
        }));
    }

    let body = records
        .iter()
        .map(|r| serde_json::to_string(r).map_err(|err| err.to_string()))
        .collect::<Result<Vec<_>, _>>()?
        .join("\n");
    std::fs::write(&cli.output_path, format!("{body}\n"))
        .map_err(|err| format!("write {}: {err}", cli.output_path.display()))?;
    eprintln!("output={}", cli.output_path.display());
    Ok(())
}

fn terminal_kind(transcript: &operator_runtime_domain::session::session_transcript::SessionTranscript) -> String {
    use operator_shared_domain::action::operator_action::OperatorAction;
    match transcript.terminal_action() {
        OperatorAction::Stop(_) => "stop".to_string(),
        OperatorAction::Escalate(_) => "escalate".to_string(),
        OperatorAction::ToolCall(_) => "tool_call".to_string(),
    }
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
