//! Smoke tests for the `operator-train` binary. The tests build the
//! binary via Cargo, point it at stub shell scripts that mimic the
//! Python trainer / predictor outputs, and verify the dispatch + arg
//! threading work end to end.

#![cfg(unix)]

use std::fs::{self, File};
use std::io::Write as _;
use std::os::unix::fs::PermissionsExt as _;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

static SEQ: AtomicU64 = AtomicU64::new(1);
static EXEC_LOCK: Mutex<()> = Mutex::new(());

fn tmp_dir(label: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let path = std::env::temp_dir().join(format!("operator-train-cli-{label}-{pid}-{n}"));
    fs::create_dir_all(&path).unwrap();
    path
}

fn tmp_file(label: &str, ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("operator-train-cli-{label}-{pid}-{n}.{ext}"))
}

fn write_stub(label: &str, body: &str) -> PathBuf {
    let path = tmp_file(label, "sh");
    {
        let mut file = File::create(&path).unwrap();
        writeln!(file, "#!/bin/sh").unwrap();
        file.write_all(body.as_bytes()).unwrap();
        file.sync_all().unwrap();
    }
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

fn cli() -> Command {
    let exe = env!("CARGO_BIN_EXE_operator-train");
    Command::new(exe)
}

#[test]
fn train_subcommand_invokes_stub_and_exits_zero() {
    let _guard = EXEC_LOCK.lock().unwrap();
    let stub = write_stub("trainok", "exit 0\n");
    let out_dir = tmp_dir("train-out");
    let status = cli()
        .args([
            "train",
            "--command",
            stub.to_str().unwrap(),
            "--base-model",
            "Qwen/Qwen2.5-0.5B-Instruct",
            "--output-dir",
            out_dir.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success(), "train must succeed against a no-op stub");
    fs::remove_file(&stub).ok();
    fs::remove_dir_all(&out_dir).ok();
}

#[test]
fn train_subcommand_propagates_non_zero_exit_code() {
    let _guard = EXEC_LOCK.lock().unwrap();
    let stub = write_stub("trainfail", "exit 7\n");
    let out_dir = tmp_dir("train-out-fail");
    let status = cli()
        .args([
            "train",
            "--command",
            stub.to_str().unwrap(),
            "--base-model",
            "Qwen/Qwen2.5-0.5B-Instruct",
            "--output-dir",
            out_dir.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(!status.success(), "train must surface the non-zero exit");
    fs::remove_file(&stub).ok();
    fs::remove_dir_all(&out_dir).ok();
}

#[test]
fn predict_subcommand_reads_stub_summary_and_exits_zero() {
    let _guard = EXEC_LOCK.lock().unwrap();
    let out_dir = tmp_dir("predict-out");
    let summary_path = out_dir.join("summary.json");
    let predictions_path = out_dir.join("predictions.jsonl");
    let body = format!(
        "cat > {} <<'EOF'\n{}\nEOF\ntouch {}\nexit 0\n",
        summary_path.to_string_lossy(),
        r#"{"predictions":2,"failures":0}"#,
        predictions_path.to_string_lossy(),
    );
    let stub = write_stub("predict", &body);
    let status = cli()
        .args([
            "predict",
            "--command",
            stub.to_str().unwrap(),
            "--base-model",
            "Qwen/Qwen2.5-0.5B-Instruct",
            "--adapter-dir",
            "/tmp/some-adapter",
            "--dataset-jsonl",
            "/tmp/some-dataset.jsonl",
            "--output-dir",
            out_dir.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    fs::remove_file(&stub).ok();
    fs::remove_dir_all(&out_dir).ok();
}

#[test]
fn validate_subcommand_passes_when_ground_truth_aligns_with_predictions() {
    let _guard = EXEC_LOCK.lock().unwrap();
    let out_dir = tmp_dir("validate-out");

    // Ground truth: one inspect trajectory targeting node:1.
    let ground_truth_path = tmp_file("truth", "jsonl");
    let trajectory_json = r#"{
        "id": "traj:1",
        "step_id": "step:1",
        "about": "about:1",
        "mode": "read",
        "task_family": "read.inspect",
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
    fs::write(&ground_truth_path, format!("{trajectory_json}\n")).unwrap();

    // The stub predictor writes a predictions.jsonl that matches the
    // ground truth perfectly + a summary with predictions=1.
    let summary_path = out_dir.join("summary.json");
    let predictions_path = out_dir.join("predictions.jsonl");
    let prediction_line = r#"{"step_id":"step:1","action":{"kind":"tool_call","tool":"kernel_inspect","arguments":{"target":"node:1"}}}"#;
    let body = format!(
        "cat > {} <<'EOF'\n{}\nEOF\ncat > {} <<'EOF'\n{}\nEOF\nexit 0\n",
        summary_path.to_string_lossy(),
        r#"{"predictions":1,"failures":0}"#,
        predictions_path.to_string_lossy(),
        prediction_line,
    );
    let stub = write_stub("predictok", &body);

    let status = cli()
        .args([
            "validate",
            "--predictor-command",
            stub.to_str().unwrap(),
            "--base-model",
            "Qwen/Qwen2.5-0.5B-Instruct",
            "--adapter-dir",
            "/tmp/some-adapter",
            "--dataset-jsonl",
            "/tmp/some-dataset.jsonl",
            "--predictor-output-dir",
            out_dir.to_str().unwrap(),
            "--ground-truth-jsonl",
            ground_truth_path.to_str().unwrap(),
            "--min-pass-rate",
            "0.5",
        ])
        .status()
        .unwrap();
    assert!(
        status.success(),
        "validate must exit 0 when the predictor matches ground truth"
    );

    fs::remove_file(&stub).ok();
    fs::remove_file(&ground_truth_path).ok();
    fs::remove_dir_all(&out_dir).ok();
}
