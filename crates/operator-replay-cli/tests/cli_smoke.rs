//! Smoke tests for the `operator-replay` binary. Spins up a tiny
//! single-request HTTP server on an ephemeral port and points the
//! CLI at it via `--mcp-endpoint`.
//!
//! Reuses the same mock-server pattern as
//! `operator-replay-infra/tests/http_adapter.rs` so the CLI is
//! exercised end-to-end without a real KMP server.

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

fn tmp(label: &str, ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("operator-replay-cli-{label}-{pid}-{n}.{ext}"))
}

fn cli() -> Command {
    let exe = env!("CARGO_BIN_EXE_operator-replay");
    Command::new(exe)
}

/// Spawn a single-request mock HTTP server that replies with the
/// given body. Returns the URL it's listening on.
fn spawn_mock_server(response_body: String) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let port = listener.local_addr().expect("local addr").port();
    let url = format!("http://127.0.0.1:{port}");
    thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept connection");
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
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body,
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response");
        stream.flush().expect("flush response");
    });
    url
}

fn write_ground_truth(path: &std::path::Path) {
    // One inspect trajectory at step:1, trajectory id traj:1.
    let row = r#"{
        "id": "traj:1",
        "step_id": "step:1",
        "about": "about:1",
        "mode": "read",
        "task_family": "read.inspect",
        "goal": "Inspect node:1.",
        "allowed_tools": [
            "kernel_wake","kernel_ask","kernel_near","kernel_goto",
            "kernel_rewind","kernel_forward","kernel_trace","kernel_inspect"
        ],
        "visible_state": {
            "known_refs": ["node:1"],
            "known_dimensions": [],
            "active_cursor": null,
            "budget": {"calls_remaining": null, "tokens_remaining": null}
        },
        "target_action": {
            "kind": "tool_call",
            "tool": "kernel_inspect",
            "arguments": {"target": "node:1"}
        }
    }"#
    .replace('\n', " ");
    fs::write(path, format!("{row}\n")).unwrap();
}

fn inspect_prediction_jsonl(path: &std::path::Path) {
    fs::write(
        path,
        r#"{"step_id":"step:1","action":{"kind":"tool_call","tool":"kernel_inspect","arguments":{"target":"node:1"}}}
"#,
    )
    .unwrap();
}

#[test]
fn successful_replay_against_mock_server_exits_zero() {
    // Canned MCP response for `kernel_inspect` — the structuredContent
    // shape the InspectResponseMapper accepts.
    let envelope = r#"{
        "jsonrpc": "2.0",
        "id": 1,
        "result": {
            "structuredContent": {
                "summary": "Found node:1",
                "object": {"ref": "node:1", "kind": "claim"},
                "links": {"incoming": [], "outgoing": []}
            }
        }
    }"#
    .to_string();
    let url = spawn_mock_server(envelope);

    let predictions = tmp("preds-ok", "jsonl");
    let ground_truth = tmp("truth-ok", "jsonl");
    inspect_prediction_jsonl(&predictions);
    write_ground_truth(&ground_truth);

    let status = cli()
        .args([
            "--predictions",
            predictions.to_str().unwrap(),
            "--ground-truth",
            ground_truth.to_str().unwrap(),
            "--mcp-endpoint",
            &url,
            "--min-success-rate",
            "1.0",
        ])
        .status()
        .unwrap();
    assert!(
        status.success(),
        "successful tool-call must pass --min-success-rate=1.0"
    );

    fs::remove_file(&predictions).ok();
    fs::remove_file(&ground_truth).ok();
}

#[test]
fn server_error_drives_replay_below_threshold_and_exits_non_zero() {
    let envelope =
        r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}"#
            .to_string();
    let url = spawn_mock_server(envelope);

    let predictions = tmp("preds-fail", "jsonl");
    let ground_truth = tmp("truth-fail", "jsonl");
    inspect_prediction_jsonl(&predictions);
    write_ground_truth(&ground_truth);

    let status = cli()
        .args([
            "--predictions",
            predictions.to_str().unwrap(),
            "--ground-truth",
            ground_truth.to_str().unwrap(),
            "--mcp-endpoint",
            &url,
            "--min-success-rate",
            "0.5",
        ])
        .status()
        .unwrap();
    assert!(
        !status.success(),
        "JSON-RPC error must drive the success rate below 0.5 and exit non-zero"
    );

    fs::remove_file(&predictions).ok();
    fs::remove_file(&ground_truth).ok();
}

#[test]
fn empty_predictions_surface_an_error() {
    let predictions = tmp("preds-empty", "jsonl");
    let ground_truth = tmp("truth-empty", "jsonl");
    fs::write(&predictions, "").unwrap();
    write_ground_truth(&ground_truth);

    let status = cli()
        .args([
            "--predictions",
            predictions.to_str().unwrap(),
            "--ground-truth",
            ground_truth.to_str().unwrap(),
            "--mcp-endpoint",
            "http://127.0.0.1:1", // never contacted
        ])
        .status()
        .unwrap();
    assert!(!status.success());

    fs::remove_file(&predictions).ok();
    fs::remove_file(&ground_truth).ok();
}
