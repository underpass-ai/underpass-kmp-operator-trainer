//! Smoke tests for `operator-llm-baseline`. Spins up an in-process
//! HTTP server that mimics the `OpenAI` chat-completions wire format,
//! drives the CLI against an ephemeral port, and inspects the
//! resulting `predictions.jsonl` / `failures.jsonl` / `summary.json`.
//!
//! The mock server accepts a fixed number of requests (one per row
//! of the SFT input) and returns canned responses in order, then
//! closes. Each test sets up its own port so they can run in parallel.

#![cfg(unix)]

use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread;
use std::time::Duration;

static SEQ: AtomicU64 = AtomicU64::new(1);

fn tmp(label: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("operator-llm-baseline-{label}-{pid}-{n}"))
}

fn cli() -> Command {
    Command::new(env!("CARGO_BIN_EXE_operator-llm-baseline"))
}

#[derive(Clone)]
struct MockResponse {
    status_line: &'static str,
    body: String,
}

fn ok(body: impl Into<String>) -> MockResponse {
    MockResponse {
        status_line: "200 OK",
        body: body.into(),
    }
}

fn server_error(body: impl Into<String>) -> MockResponse {
    MockResponse {
        status_line: "500 Internal Server Error",
        body: body.into(),
    }
}

/// Spawn an HTTP server that serves `responses.len()` requests in
/// order, then closes. Returns the base URL (without trailing path).
fn spawn_mock_server(responses: Vec<MockResponse>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let port = listener.local_addr().expect("local addr").port();
    let url = format!("http://127.0.0.1:{port}");
    thread::spawn(move || {
        for response in responses {
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .expect("set read timeout");
            let mut reader = BufReader::new(&mut stream);
            let mut content_length: usize = 0;
            loop {
                let mut line = String::new();
                let bytes = reader.read_line(&mut line).expect("read header");
                if bytes == 0 || line == "\r\n" || line == "\n" {
                    break;
                }
                let lower = line.to_ascii_lowercase();
                if let Some(rest) = lower.strip_prefix("content-length:") {
                    content_length = rest.trim().parse().unwrap_or(0);
                }
            }
            let mut body_bytes = vec![0u8; content_length];
            reader.read_exact(&mut body_bytes).expect("read body");
            drop(reader);
            let http_response = format!(
                "HTTP/1.1 {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                response.status_line,
                response.body.len(),
                response.body,
            );
            stream
                .write_all(http_response.as_bytes())
                .expect("write response");
            stream.flush().expect("flush response");
        }
    });
    url
}

fn inspect_sft_row() -> String {
    // One row with system + user + assistant; the baseliner sends only
    // the first two to the LLM.
    let row = serde_json::json!({
        "step_id": "step:1",
        "messages": [
            {"role": "system", "content": "You are the kernel operator."},
            {"role": "user", "content": "{\"about\":\"about:1\",\"allowed_tools\":[\"kernel_inspect\"]}"},
            {"role": "assistant", "content": "{\"action\":{\"kind\":\"tool_call\",\"tool\":\"kernel_inspect\",\"arguments\":{\"target\":\"node:1\"}}}"}
        ]
    });
    serde_json::to_string(&row).unwrap()
}

fn chat_completion_envelope(content: &str) -> String {
    let envelope = serde_json::json!({
        "id": "chatcmpl-test",
        "object": "chat.completion",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": content},
            "finish_reason": "stop"
        }]
    });
    envelope.to_string()
}

#[test]
fn happy_path_writes_predictions_and_succeeds() {
    let sft = tmp("happy-sft");
    fs::write(&sft, format!("{}\n", inspect_sft_row())).unwrap();

    let output = tmp("happy-out");
    let action_payload = r#"{"action":{"kind":"tool_call","tool":"kernel_inspect","arguments":{"target":"node:1"}}}"#;
    let url = spawn_mock_server(vec![ok(chat_completion_envelope(action_payload))]);

    let status = cli()
        .args([
            "--sft",
            sft.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--api-base",
            &url,
            "--model",
            "test-model",
            "--max-failures",
            "0",
        ])
        .status()
        .unwrap();
    assert!(
        status.success(),
        "happy path must exit zero with --max-failures=0"
    );

    let predictions = fs::read_to_string(output.join("predictions.jsonl")).unwrap();
    assert!(
        predictions.contains("\"step_id\":\"step:1\""),
        "predictions.jsonl missing expected step_id: {predictions}"
    );
    assert!(
        predictions.contains("\"tool\":\"kernel_inspect\""),
        "predictions.jsonl missing expected tool: {predictions}"
    );
    let failures = fs::read_to_string(output.join("failures.jsonl")).unwrap();
    assert!(
        failures.trim().is_empty(),
        "expected no failures: {failures}"
    );
    let summary = fs::read_to_string(output.join("summary.json")).unwrap();
    assert!(summary.contains("\"succeeded\": 1"));
    assert!(summary.contains("\"failed\": 0"));

    fs::remove_dir_all(&output).ok();
    fs::remove_file(&sft).ok();
}

#[test]
fn bare_action_payload_is_accepted() {
    let sft = tmp("bare-sft");
    fs::write(&sft, format!("{}\n", inspect_sft_row())).unwrap();

    let output = tmp("bare-out");
    let bare_action =
        r#"{"kind":"tool_call","tool":"kernel_inspect","arguments":{"target":"node:1"}}"#;
    let url = spawn_mock_server(vec![ok(chat_completion_envelope(bare_action))]);

    let status = cli()
        .args([
            "--sft",
            sft.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--api-base",
            &url,
            "--model",
            "test-model",
            "--max-failures",
            "0",
        ])
        .status()
        .unwrap();
    assert!(status.success(), "bare OperatorActionDto must be accepted");

    let predictions = fs::read_to_string(output.join("predictions.jsonl")).unwrap();
    assert!(predictions.contains("\"tool\":\"kernel_inspect\""));

    fs::remove_dir_all(&output).ok();
    fs::remove_file(&sft).ok();
}

#[test]
fn malformed_response_is_recorded_as_failure() {
    let sft = tmp("malformed-sft");
    fs::write(&sft, format!("{}\n", inspect_sft_row())).unwrap();

    let output = tmp("malformed-out");
    let url = spawn_mock_server(vec![ok(chat_completion_envelope(
        "this is not valid JSON for an action",
    ))]);

    let status = cli()
        .args([
            "--sft",
            sft.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--api-base",
            &url,
            "--model",
            "test-model",
            "--max-failures",
            "0",
        ])
        .status()
        .unwrap();
    assert!(
        !status.success(),
        "malformed response should breach --max-failures=0"
    );

    let predictions = fs::read_to_string(output.join("predictions.jsonl")).unwrap();
    assert!(
        predictions.trim().is_empty(),
        "predictions.jsonl must be empty on parse failure: {predictions}"
    );
    let failures = fs::read_to_string(output.join("failures.jsonl")).unwrap();
    assert!(
        failures.contains("\"step_id\":\"step:1\""),
        "failures.jsonl missing step row: {failures}"
    );

    fs::remove_dir_all(&output).ok();
    fs::remove_file(&sft).ok();
}

#[test]
fn http_500_is_recorded_as_failure() {
    let sft = tmp("http500-sft");
    fs::write(&sft, format!("{}\n", inspect_sft_row())).unwrap();

    let output = tmp("http500-out");
    let url = spawn_mock_server(vec![server_error(
        r#"{"error":{"message":"rate limited","code":"rate_limit_exceeded"}}"#,
    )]);

    let status = cli()
        .args([
            "--sft",
            sft.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--api-base",
            &url,
            "--model",
            "test-model",
        ])
        .status()
        .unwrap();
    // Without --max-failures, the CLI still exits zero — we only
    // record the failure. The test is asserting the bookkeeping
    // shape, not the gate.
    assert!(status.success());
    let failures = fs::read_to_string(output.join("failures.jsonl")).unwrap();
    assert!(
        failures.contains("rate_limit_exceeded") || failures.contains("rate limited"),
        "failures.jsonl missing API-level error detail: {failures}"
    );
    let summary = fs::read_to_string(output.join("summary.json")).unwrap();
    assert!(summary.contains("\"failed\": 1"));

    fs::remove_dir_all(&output).ok();
    fs::remove_file(&sft).ok();
}

#[test]
fn empty_sft_surface_an_error() {
    let sft = tmp("empty-sft");
    fs::write(&sft, "").unwrap();
    let output = tmp("empty-out");

    let status = cli()
        .args([
            "--sft",
            sft.to_str().unwrap(),
            "--output",
            output.to_str().unwrap(),
            "--api-base",
            "http://127.0.0.1:1",
            "--model",
            "test-model",
        ])
        .status()
        .unwrap();
    assert!(!status.success(), "empty SFT input must exit non-zero");

    fs::remove_dir_all(&output).ok();
    fs::remove_file(&sft).ok();
}
