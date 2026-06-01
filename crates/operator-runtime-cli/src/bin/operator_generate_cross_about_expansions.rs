//! Generate cross-about count SFT trajectories by driving the multi-step loop
//! with a teacher policy against a live (or replay-backed) KMP endpoint.
//!
//! For each authored cross-about episode the generator compiles a read session
//! spanning several abouts, lets the teacher demonstrate the policy (wake into
//! each about, widen its window until covered, then stop with the union),
//! verifies that every about's gold operands were retrieved, and expands the
//! accepted transcripts into `{prompt, completion}` SFT rows attributed to the
//! about current at each step. A drop-rate gate fails the run if too many
//! episodes left an about short of coverage.
//!
//! The teacher and the KMP executor are wired from flags, so the same binary
//! runs against a live mTLS-fronted kernel (via the `rehydration-mcp` stdio
//! bridge) or a local/replay endpoint without code changes.

use std::path::PathBuf;
use std::sync::Arc;

use clap::{Parser, ValueEnum};
use operator_runtime_application::ports::mcp_executor_port::McpExecutor;
use operator_runtime_application::ports::operator_policy_port::OperatorPolicy;
use operator_runtime_application::use_cases::generate_cross_about_expansions_use_case::GenerateCrossAboutExpansionsUseCase;
use operator_runtime_application::use_cases::run_operator_session_multi_step_use_case::RunOperatorSessionMultiStepUseCase;
use operator_runtime_infra::adapters::kmp_mcp_http_executor::KmpMcpHttpExecutor;
use operator_runtime_infra::adapters::kmp_mcp_stdio_config::KmpMcpStdioConfig;
use operator_runtime_infra::adapters::kmp_mcp_stdio_executor::KmpMcpStdioExecutor;
use operator_runtime_infra::adapters::stderr_session_event_sink::StderrSessionEventSink;
use operator_runtime_infra::adapters::teacher_backed_operator_policy::TeacherBackedOperatorPolicy;
use operator_shared_domain::contract::composite_action_contract_validator::CompositeActionContractValidator;
use operator_shared_domain::value_objects::task_family::TaskFamily;
use operator_synthetic_infra::adapters::jsonl_cross_about_episode_source::JsonlCrossAboutEpisodeSource;
use operator_synthetic_infra::adapters::openai_compatible_teacher_policy::OpenAiCompatibleTeacherPolicy;
use operator_training_application::ports::dataset_writer::DatasetWriter;
use operator_training_infra::adapters::jsonl_sft_dataset_writer::JsonlSftDatasetWriter;
use operator_training_infra::adapters::jsonl_trajectory_dataset_writer::JsonlTrajectoryDatasetWriter;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
enum KmpMcpTransport {
    Stdio,
    Http,
}

#[derive(Debug, Parser)]
#[command(name = "operator-generate-cross-about-expansions")]
#[command(about = "Generate cross-about count SFT trajectories with a teacher policy")]
struct Cli {
    #[arg(long)]
    episodes_jsonl: PathBuf,
    #[arg(long)]
    output_dir: PathBuf,
    #[arg(long, default_value = "runtime.cross_about_count")]
    task_family: String,
    #[arg(long, default_value_t = 0.5)]
    max_drop_rate: f64,
    #[arg(long)]
    limit: Option<usize>,

    // Teacher (OpenAI-compatible endpoint; the demonstrator the student learns from).
    #[arg(long)]
    teacher_api_base: String,
    #[arg(long)]
    teacher_model: String,
    #[arg(long)]
    teacher_prompt: PathBuf,
    /// Teacher API key. Falls back to the `OPERATOR_TEACHER_API_KEY` env var.
    #[arg(long)]
    teacher_api_key: Option<String>,

    // KMP executor.
    #[arg(long, alias = "kmp-grpc-endpoint")]
    kmp_mcp_endpoint: String,
    #[arg(long, value_enum, default_value_t = KmpMcpTransport::Stdio)]
    kmp_mcp_transport: KmpMcpTransport,
    #[arg(long, default_value = "rehydration-mcp")]
    kmp_mcp_stdio_command: PathBuf,
}

fn main() {
    if let Err(err) = run(&Cli::parse()) {
        eprintln!("operator-generate-cross-about-expansions failed: {err}");
        std::process::exit(1);
    }
}

fn run(cli: &Cli) -> Result<(), String> {
    let task_family = TaskFamily::parse(cli.task_family.as_str()).map_err(|err| err.to_string())?;

    let mut source = JsonlCrossAboutEpisodeSource::new(&cli.episodes_jsonl);
    if let Some(limit) = cli.limit {
        source = source.with_limit(limit);
    }
    let episodes = source.read().map_err(|err| err.to_string())?;

    let api_key = cli
        .teacher_api_key
        .clone()
        .or_else(|| std::env::var("OPERATOR_TEACHER_API_KEY").ok());
    let teacher = OpenAiCompatibleTeacherPolicy::new(
        cli.teacher_api_base.as_str(),
        api_key,
        cli.teacher_model.as_str(),
        cli.teacher_prompt.as_path(),
    )
    .map_err(|err| err.to_string())?;
    let policy: Arc<dyn OperatorPolicy> = Arc::new(TeacherBackedOperatorPolicy::new(teacher));

    let session = RunOperatorSessionMultiStepUseCase::new(
        policy,
        build_mcp_executor(cli)?,
        CompositeActionContractValidator::default_strict(),
        Arc::new(StderrSessionEventSink::new()),
    );
    let generator = GenerateCrossAboutExpansionsUseCase::new(session, task_family);
    let report = generator.execute(&episodes).map_err(|err| err.to_string())?;

    std::fs::create_dir_all(&cli.output_dir)
        .map_err(|err| format!("create output dir {}: {err}", cli.output_dir.display()))?;
    let dataset_path = cli.output_dir.join("cross_about_expansions.sft.jsonl");
    JsonlSftDatasetWriter::new(&dataset_path)
        .write(report.trajectories())
        .map_err(|err| err.to_string())?;

    // Rich trajectory file (about/goal/mode/task_family/allowed_tools/
    // visible_state/target_action) — the schema prepare_operator_sft_dataset.py
    // --trajectories consumes to build the {system,user,assistant} chat dataset.
    let trajectory_path = cli.output_dir.join("cross_about_trajectories.jsonl");
    JsonlTrajectoryDatasetWriter::new(&trajectory_path)
        .write(report.trajectories())
        .map_err(|err| err.to_string())?;

    eprintln!(
        "episodes={} accepted={} dropped={} drop_rate={:.3} trajectories={} output={}",
        report.total_episodes(),
        report.accepted_episodes(),
        report.dropped_episodes(),
        report.drop_rate(),
        report.trajectories().len(),
        dataset_path.display(),
    );
    for drop in report.drops() {
        eprintln!(
            "  dropped entry_about={} reason={}",
            drop.entry_about(),
            drop.reason()
        );
    }

    if report.drop_rate() > cli.max_drop_rate {
        return Err(format!(
            "drop rate {:.3} exceeds maximum {:.3}",
            report.drop_rate(),
            cli.max_drop_rate
        ));
    }
    Ok(())
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
