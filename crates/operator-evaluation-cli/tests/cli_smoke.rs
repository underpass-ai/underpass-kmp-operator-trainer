//! Smoke tests for the `operator-policy-eval` binary.

#![cfg(unix)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static SEQ: AtomicU64 = AtomicU64::new(1);

fn tmp(label: &str, ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("operator-policy-eval-{label}-{pid}-{n}.{ext}"))
}

fn cli() -> Command {
    let exe = env!("CARGO_BIN_EXE_operator-policy-eval");
    Command::new(exe)
}

fn write_ground_truth(path: &std::path::Path) {
    // One inspect trajectory targeting node:1.
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

#[test]
fn exact_match_predictions_pass_the_threshold() {
    let predictions = tmp("preds-ok", "jsonl");
    let ground_truth = tmp("truth-ok", "jsonl");
    write_ground_truth(&ground_truth);
    fs::write(
        &predictions,
        r#"{"step_id":"step:1","action":{"kind":"tool_call","tool":"kernel_inspect","arguments":{"target":"node:1"}}}
"#,
    )
    .unwrap();

    let status = cli()
        .args([
            "--predictions",
            predictions.to_str().unwrap(),
            "--ground-truth",
            ground_truth.to_str().unwrap(),
            "--min-pass-rate",
            "0.5",
        ])
        .status()
        .unwrap();
    assert!(
        status.success(),
        "exact-match prediction must pass --min-pass-rate=0.5"
    );

    fs::remove_file(&predictions).ok();
    fs::remove_file(&ground_truth).ok();
}

#[test]
fn output_includes_legacy_and_action_correctness_sections() {
    let predictions = tmp("preds-sections", "jsonl");
    let ground_truth = tmp("truth-sections", "jsonl");
    write_ground_truth(&ground_truth);
    fs::write(
        &predictions,
        r#"{"step_id":"step:1","action":{"kind":"tool_call","tool":"kernel_inspect","arguments":{"target":"node:1"}}}
"#,
    )
    .unwrap();

    let output = cli()
        .args([
            "--predictions",
            predictions.to_str().unwrap(),
            "--ground-truth",
            ground_truth.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("=== Legacy metrics ==="));
    assert!(stdout.contains("=== Action correctness (v8.1) ==="));
    assert!(stdout.contains("action_correct:"));
    assert!(stdout.contains("Per-field correctness"));

    fs::remove_file(&predictions).ok();
    fs::remove_file(&ground_truth).ok();
}

#[test]
fn wrong_target_fails_the_threshold() {
    let predictions = tmp("preds-bad", "jsonl");
    let ground_truth = tmp("truth-bad", "jsonl");
    write_ground_truth(&ground_truth);
    // Predicts node:42 but ground truth is node:1 → exact match rate 0.0.
    fs::write(
        &predictions,
        r#"{"step_id":"step:1","action":{"kind":"tool_call","tool":"kernel_inspect","arguments":{"target":"node:42"}}}
"#,
    )
    .unwrap();

    let status = cli()
        .args([
            "--predictions",
            predictions.to_str().unwrap(),
            "--ground-truth",
            ground_truth.to_str().unwrap(),
            "--min-pass-rate",
            "0.5",
        ])
        .status()
        .unwrap();
    assert!(
        !status.success(),
        "off-target prediction must fail --min-pass-rate=0.5"
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
        ])
        .status()
        .unwrap();
    assert!(!status.success(), "empty predictions must fail loudly");

    fs::remove_file(&predictions).ok();
    fs::remove_file(&ground_truth).ok();
}

#[test]
fn no_step_id_overlap_surfaces_an_error() {
    let predictions = tmp("preds-noover", "jsonl");
    let ground_truth = tmp("truth-noover", "jsonl");
    write_ground_truth(&ground_truth);
    fs::write(
        &predictions,
        r#"{"step_id":"step:nonexistent","action":{"kind":"tool_call","tool":"kernel_inspect","arguments":{"target":"node:1"}}}
"#,
    )
    .unwrap();

    let status = cli()
        .args([
            "--predictions",
            predictions.to_str().unwrap(),
            "--ground-truth",
            ground_truth.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(
        !status.success(),
        "no step_id overlap must fail with a clear error"
    );

    fs::remove_file(&predictions).ok();
    fs::remove_file(&ground_truth).ok();
}
