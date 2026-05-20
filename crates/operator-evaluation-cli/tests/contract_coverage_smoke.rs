//! Smoke tests for the `operator-contract-coverage` binary.

#![cfg(unix)]

use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

static SEQ: AtomicU64 = AtomicU64::new(1);

fn tmp(label: &str, ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    std::env::temp_dir().join(format!(
        "operator-contract-coverage-{label}-{pid}-{n}.{ext}"
    ))
}

fn cli() -> Command {
    let exe = env!("CARGO_BIN_EXE_operator-contract-coverage");
    Command::new(exe)
}

/// One canonical inspect trajectory — single capability, valid action.
fn write_single_inspect(path: &std::path::Path) {
    let row = r#"{
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
    fs::write(path, format!("{row}\n")).unwrap();
}

/// Synthesize a full-coverage JSONL by invoking the sibling
/// `operator-synthesize` binary so the test rides the same path the
/// production runbook describes.
fn synthesize_full_coverage(output: &std::path::Path) {
    // Cargo's CARGO_BIN_EXE_* env vars only point at bins of the
    // crate under test, so we exec the binary via `cargo run` to
    // pull in the workspace's synthesizer without coupling this
    // crate's deps to `operator-synthetic-cli`.
    let status = Command::new(env!("CARGO"))
        .args([
            "run",
            "--quiet",
            "-p",
            "operator-synthetic-cli",
            "--bin",
            "operator-synthesize",
            "--",
            "--dataset-id",
            "dataset:cc-smoke",
            "--minimum-examples",
            "1",
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .expect("invoke operator-synthesize");
    assert!(status.success(), "synthesize must succeed");
}

#[test]
fn single_trajectory_reports_partial_coverage() {
    let trajectories = tmp("single", "jsonl");
    write_single_inspect(&trajectories);
    let status = cli()
        .args(["--trajectories", trajectories.to_str().unwrap()])
        .status()
        .unwrap();
    // No gate flags → exit 0 regardless of coverage.
    assert!(status.success());
    fs::remove_file(&trajectories).ok();
}

#[test]
fn single_trajectory_fails_require_full_coverage() {
    let trajectories = tmp("partial", "jsonl");
    write_single_inspect(&trajectories);
    let status = cli()
        .args([
            "--trajectories",
            trajectories.to_str().unwrap(),
            "--require-full-coverage",
        ])
        .status()
        .unwrap();
    assert!(
        !status.success(),
        "single-tool dataset must fail --require-full-coverage"
    );
    fs::remove_file(&trajectories).ok();
}

#[test]
fn synthesized_dataset_passes_require_full_coverage() {
    let trajectories = tmp("full", "jsonl");
    synthesize_full_coverage(&trajectories);
    let status = cli()
        .args([
            "--trajectories",
            trajectories.to_str().unwrap(),
            "--require-full-coverage",
            "--require-zero-invalid",
        ])
        .status()
        .unwrap();
    assert!(
        status.success(),
        "synthesized dataset must cover every KernelTool AND validate"
    );
    fs::remove_file(&trajectories).ok();
}

#[test]
fn empty_jsonl_surfaces_an_error() {
    let trajectories = tmp("empty", "jsonl");
    fs::write(&trajectories, "").unwrap();
    let status = cli()
        .args(["--trajectories", trajectories.to_str().unwrap()])
        .status()
        .unwrap();
    assert!(!status.success());
    fs::remove_file(&trajectories).ok();
}

#[test]
fn nonexistent_path_surfaces_an_error() {
    let status = cli()
        .args(["--trajectories", "/no/such/path/trajectories.jsonl"])
        .status()
        .unwrap();
    assert!(!status.success());
}
