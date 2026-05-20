//! `operator-llm-baseline` — generate `predictions.jsonl` by calling
//! a frontier LLM over the same SFT JSONL the fine-tuned predictor
//! reads.
//!
//! For each row in the SFT dataset:
//!   1. drop the assistant message (`messages[:-1]`),
//!   2. send the remaining `system + user` messages to a chat-completions
//!      endpoint via [`OpenAiCompatibleLlmBaseliner`],
//!   3. parse the assistant content as either
//!      `{"action": <OperatorActionDto>}` (preferred, matches the SFT
//!      assistant target shape) or as a bare `OperatorActionDto`,
//!   4. write `{step_id, action}` to `predictions.jsonl`.
//!
//! Rows that fail (HTTP error, malformed JSON, missing action field)
//! are recorded in `failures.jsonl` with the raw response, so the run
//! can be triaged later. The output directory and summary mirror
//! `predict_operator_sft.py` so the resulting `predictions.jsonl`
//! drops straight into `operator-policy-eval`.

use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::Parser;
use operator_evaluation_application::error::llm_baseliner_error::LlmBaselinerError;
use operator_evaluation_application::ports::chat_message::ChatMessage;
use operator_evaluation_application::ports::llm_baseliner::LlmBaseliner;
use operator_evaluation_infra::adapters::openai_compatible_llm_baseliner::OpenAiCompatibleLlmBaseliner;
use operator_shared_contract::operator_action_dto::OperatorActionDto;
use serde::{Deserialize, Serialize};

const PREDICTOR_NAME: &str = "operator-llm-baseline-v1";

#[derive(Parser, Debug)]
#[command(
    name = "operator-llm-baseline",
    about = "Call a frontier LLM over an SFT JSONL and emit predictions.jsonl.",
    version
)]
struct Cli {
    /// SFT JSONL produced by `prepare_operator_sft.py` (or equivalent),
    /// one `{step_id, messages: [...]}` per line.
    #[arg(long)]
    sft: PathBuf,
    /// Output directory. Created if missing. Files written:
    /// `predictions.jsonl`, `failures.jsonl`, `summary.json`.
    #[arg(long)]
    output: PathBuf,
    /// OpenAI-compatible base URL (no `/chat/completions` suffix).
    /// Examples: `https://api.openai.com/v1`,
    /// `https://llm.underpassai.com/v1`,
    /// `https://api.anthropic.com/v1`.
    #[arg(long)]
    api_base: String,
    /// Path to a file containing the bearer token (the file is read
    /// once and trimmed). Omit for endpoints that do not require
    /// authentication (e.g., a local vLLM without `--api-key`).
    #[arg(long)]
    api_key_file: Option<PathBuf>,
    /// Model identifier accepted by the endpoint (e.g.,
    /// `gpt-4o-mini`, `claude-opus-4-5`, `Qwen2.5-7B-Instruct`).
    #[arg(long)]
    model: String,
    /// Sampling temperature. Defaults to `0.0` so a baseline run is
    /// reproducible.
    #[arg(long, default_value_t = 0.0)]
    temperature: f32,
    /// Optional `max_tokens` cap. Leave unset to use the server
    /// default.
    #[arg(long)]
    max_tokens: Option<u32>,
    /// Cap on rows processed; useful for smoke runs against paid
    /// endpoints. Defaults to no cap.
    #[arg(long)]
    limit: Option<usize>,
    /// If set, the CLI exits non-zero when the number of failed rows
    /// exceeds this threshold.
    #[arg(long)]
    max_failures: Option<usize>,
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
            eprintln!("llm-baseline failed: {err}");
            ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<bool, CliError> {
    fs::create_dir_all(&cli.output)
        .map_err(|err| CliError::Generic(format!("create output dir: {err}")))?;

    let rows = read_sft_rows(&cli.sft)?;
    if rows.is_empty() {
        return Err(CliError::Generic(
            "SFT JSONL is empty; nothing to baseline".into(),
        ));
    }
    let rows = match cli.limit {
        Some(cap) => rows.into_iter().take(cap).collect::<Vec<_>>(),
        None => rows,
    };

    let api_key = load_api_key(cli.api_key_file.as_deref())?;
    let mut baseliner = OpenAiCompatibleLlmBaseliner::new(&cli.api_base, api_key, &cli.model)
        .map_err(|err| CliError::Generic(format!("build baseliner: {err}")))?
        .with_temperature(cli.temperature);
    if let Some(max_tokens) = cli.max_tokens {
        baseliner = baseliner.with_max_tokens(max_tokens);
    }

    let started_at_unix = unix_now();
    let outcome = baseline_rows(&baseliner, &rows, &cli.output)?;
    let finished_at_unix = unix_now();

    let summary = Summary {
        predictor: PREDICTOR_NAME.to_string(),
        api_base: cli.api_base.clone(),
        model: cli.model.clone(),
        temperature: cli.temperature,
        max_tokens: cli.max_tokens,
        dataset: cli.sft.display().to_string(),
        selected: rows.len(),
        succeeded: outcome.succeeded,
        failed: outcome.failed,
        started_at_unix,
        finished_at_unix,
    };
    let summary_text = serde_json::to_string_pretty(&summary)
        .map_err(|err| CliError::Generic(format!("serialise summary: {err}")))?;
    fs::write(cli.output.join("summary.json"), format!("{summary_text}\n"))
        .map_err(|err| CliError::Generic(format!("write summary.json: {err}")))?;
    println!("{summary_text}");

    let passed = match cli.max_failures {
        Some(threshold) => outcome.failed <= threshold,
        None => true,
    };
    Ok(passed)
}

struct BatchOutcome {
    succeeded: usize,
    failed: usize,
}

fn baseline_rows(
    baseliner: &dyn LlmBaseliner,
    rows: &[SftRow],
    output_dir: &Path,
) -> Result<BatchOutcome, CliError> {
    let predictions_file = File::create(output_dir.join("predictions.jsonl"))
        .map_err(|err| CliError::Generic(format!("create predictions.jsonl: {err}")))?;
    let failures_file = File::create(output_dir.join("failures.jsonl"))
        .map_err(|err| CliError::Generic(format!("create failures.jsonl: {err}")))?;
    let mut predictions = BufWriter::new(predictions_file);
    let mut failures = BufWriter::new(failures_file);

    let mut succeeded = 0usize;
    let mut failed = 0usize;

    for (index, row) in rows.iter().enumerate() {
        match process_row(baseliner, row) {
            RowOutcome::Success { action } => {
                write_jsonl(
                    &mut predictions,
                    &PredictionRow {
                        step_id: row.step_id.clone(),
                        action,
                    },
                    "prediction",
                )?;
                succeeded += 1;
            }
            RowOutcome::Failure { reason, raw } => {
                write_jsonl(
                    &mut failures,
                    &FailureRow {
                        step_id: row.step_id.clone(),
                        reason,
                        raw_response: raw,
                    },
                    "failure",
                )?;
                failed += 1;
            }
        }

        if (index + 1).is_multiple_of(10) || index + 1 == rows.len() {
            eprintln!(
                "{}",
                serde_json::json!({
                    "event": "operator_llm_baseline.progress",
                    "completed": index + 1,
                    "total": rows.len(),
                    "succeeded": succeeded,
                    "failed": failed,
                })
            );
        }
    }

    // Flush the BufWriter, recover the inner File, and sync_all to
    // disk so paid LLM-call results survive a crash. Mirrors the
    // durability pattern from operator-synthesize.
    flush_and_sync(predictions, "predictions.jsonl")?;
    flush_and_sync(failures, "failures.jsonl")?;

    Ok(BatchOutcome { succeeded, failed })
}

fn flush_and_sync(writer: BufWriter<File>, label: &'static str) -> Result<(), CliError> {
    let file = writer
        .into_inner()
        .map_err(|err| CliError::Generic(format!("flush {label}: {err}")))?;
    file.sync_all()
        .map_err(|err| CliError::Generic(format!("sync_all {label}: {err}")))?;
    Ok(())
}

fn write_jsonl<T: Serialize>(
    writer: &mut BufWriter<File>,
    value: &T,
    label: &'static str,
) -> Result<(), CliError> {
    let line = serde_json::to_string(value)
        .map_err(|err| CliError::Generic(format!("serialise {label}: {err}")))?;
    writer
        .write_all(line.as_bytes())
        .map_err(|err| CliError::Generic(format!("write {label}: {err}")))?;
    writer
        .write_all(b"\n")
        .map_err(|err| CliError::Generic(format!("write {label}: {err}")))?;
    Ok(())
}

fn process_row(baseliner: &dyn LlmBaseliner, row: &SftRow) -> RowOutcome {
    let messages = match build_messages(row) {
        Ok(messages) => messages,
        Err(reason) => {
            return RowOutcome::Failure {
                reason,
                raw: String::new(),
            };
        }
    };

    let raw = match baseliner.complete(&messages) {
        Ok(content) => content,
        Err(err) => {
            return RowOutcome::Failure {
                reason: render_baseliner_error(&err),
                raw: String::new(),
            };
        }
    };

    match parse_action_payload(&raw) {
        Ok(action) => RowOutcome::Success { action },
        Err(reason) => RowOutcome::Failure { reason, raw },
    }
}

fn build_messages(row: &SftRow) -> Result<Vec<ChatMessage>, String> {
    if row.messages.len() < 2 {
        return Err(format!(
            "row {} has fewer than 2 messages; need at least system + user",
            row.step_id
        ));
    }
    let take = row.messages.len() - 1;
    let mut out = Vec::with_capacity(take);
    for (index, message) in row.messages.iter().take(take).enumerate() {
        let chat = ChatMessage::new(&message.role, &message.content)
            .map_err(|err| format!("row {} message {index}: {err}", row.step_id))?;
        out.push(chat);
    }
    Ok(out)
}

/// Accept either `{"action": {...}}` (the SFT assistant-target shape)
/// or a bare `OperatorActionDto`. Frontier LLMs sometimes return the
/// action directly without the wrapping object even when asked for it.
fn parse_action_payload(raw: &str) -> Result<OperatorActionDto, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("LLM returned empty content".to_string());
    }
    if let Ok(wrapper) = serde_json::from_str::<ActionWrapperDto>(trimmed) {
        return Ok(wrapper.action);
    }
    serde_json::from_str::<OperatorActionDto>(trimmed)
        .map_err(|err| format!("response is not a valid OperatorActionDto: {err}"))
}

fn render_baseliner_error(err: &LlmBaselinerError) -> String {
    match err {
        LlmBaselinerError::Transport { message, .. } => format!("transport: {message}"),
        LlmBaselinerError::Protocol { message, .. } => format!("protocol: {message}"),
        LlmBaselinerError::ApiError { code, message, .. } => match code {
            Some(code) => format!("api_error[{code}]: {message}"),
            None => format!("api_error: {message}"),
        },
    }
}

fn load_api_key(path: Option<&Path>) -> Result<Option<String>, CliError> {
    let Some(path) = path else {
        return Ok(None);
    };
    let contents = fs::read_to_string(path)
        .map_err(|err| CliError::Generic(format!("read api-key-file: {err}")))?;
    let trimmed = contents.trim();
    if trimmed.is_empty() {
        return Err(CliError::Generic(
            "api-key-file is empty after trimming whitespace".into(),
        ));
    }
    Ok(Some(trimmed.to_string()))
}

fn read_sft_rows(path: &Path) -> Result<Vec<SftRow>, CliError> {
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
        let row: SftRow = serde_json::from_str(&line)
            .map_err(|err| CliError::Generic(format!("parse SFT line {}: {err}", index + 1)))?;
        out.push(row);
    }
    Ok(out)
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs())
}

enum RowOutcome {
    Success { action: OperatorActionDto },
    Failure { reason: String, raw: String },
}

#[derive(Debug, Deserialize)]
struct SftRow {
    step_id: String,
    messages: Vec<SftMessage>,
}

#[derive(Debug, Deserialize)]
struct SftMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ActionWrapperDto {
    action: OperatorActionDto,
}

#[derive(Debug, Serialize)]
struct PredictionRow {
    step_id: String,
    action: OperatorActionDto,
}

#[derive(Debug, Serialize)]
struct FailureRow {
    step_id: String,
    reason: String,
    raw_response: String,
}

#[derive(Debug, Serialize)]
struct Summary {
    predictor: String,
    api_base: String,
    model: String,
    temperature: f32,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_tokens: Option<u32>,
    dataset: String,
    selected: usize,
    succeeded: usize,
    failed: usize,
    started_at_unix: u64,
    finished_at_unix: u64,
}

#[derive(Debug, thiserror::Error)]
enum CliError {
    #[error("{0}")]
    Generic(String),
}
