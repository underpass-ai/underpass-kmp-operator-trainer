//! Smoke tests for the `operator-synthesize` binary. Verify the
//! produced JSONL is consumable by the rest of the operator pipeline
//! (round-trips through `TrainingTrajectoryMapper::to_domain`) and
//! that all generated `step_ids` are globally unique — the property
//! every downstream join-by-`step_id` consumer relies on.

#![cfg(unix)]

use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

use operator_shared_contract::training_trajectory_dto::TrainingTrajectoryDto;
use operator_shared_infra::mappers::training_trajectory_mapper::TrainingTrajectoryMapper;

static SEQ: AtomicU64 = AtomicU64::new(1);

fn tmp(label: &str, ext: &str) -> PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    std::env::temp_dir().join(format!("operator-synthesize-{label}-{pid}-{n}.{ext}"))
}

fn cli() -> Command {
    let exe = env!("CARGO_BIN_EXE_operator-synthesize");
    Command::new(exe)
}

#[test]
fn produces_n_times_capabilities_rows_and_exits_zero() {
    let output = tmp("happy", "jsonl");
    let status = cli()
        .args([
            "--dataset-id",
            "dataset:smoke",
            "--minimum-examples",
            "2",
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success(), "CLI must exit 0 on the happy path");

    let body = fs::read_to_string(&output).expect("read JSONL");
    let lines: Vec<&str> = body.lines().filter(|l| !l.trim().is_empty()).collect();
    // The blueprint covers every `KmpMcpCapability` variant. We
    // don't hard-code the count here — the assertion is parametric
    // on 2 examples per capability.
    assert!(
        lines.len().is_multiple_of(2) && !lines.is_empty(),
        "row count must be a positive multiple of --minimum-examples=2 (got {})",
        lines.len()
    );

    fs::remove_file(&output).ok();
}

#[test]
fn every_row_round_trips_through_the_shared_mapper() {
    let output = tmp("rt", "jsonl");
    cli()
        .args([
            "--dataset-id",
            "dataset:rt",
            "--minimum-examples",
            "1",
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    let body = fs::read_to_string(&output).expect("read JSONL");
    for (index, line) in body.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let dto: TrainingTrajectoryDto = serde_json::from_str(line)
            .unwrap_or_else(|err| panic!("line {} bad JSON: {err}", index + 1));
        TrainingTrajectoryMapper::to_domain(&dto)
            .unwrap_or_else(|err| panic!("line {} fails domain mapping: {err}", index + 1));
    }
    fs::remove_file(&output).ok();
}

#[test]
fn step_ids_are_globally_unique_across_capabilities() {
    let output = tmp("step", "jsonl");
    cli()
        .args([
            "--dataset-id",
            "dataset:step",
            "--minimum-examples",
            "3",
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    let body = fs::read_to_string(&output).expect("read JSONL");
    let mut step_ids: HashSet<String> = HashSet::new();
    let mut total = 0usize;
    for line in body.lines() {
        if line.trim().is_empty() {
            continue;
        }
        total += 1;
        let dto: TrainingTrajectoryDto = serde_json::from_str(line).expect("dto parses");
        assert!(
            step_ids.insert(dto.step_id.clone()),
            "step_id collision: {}",
            dto.step_id
        );
    }
    assert_eq!(
        step_ids.len(),
        total,
        "every row must have a unique step_id"
    );
    fs::remove_file(&output).ok();
}

#[test]
fn output_ends_with_a_newline() {
    let output = tmp("nl", "jsonl");
    cli()
        .args([
            "--dataset-id",
            "dataset:nl",
            "--minimum-examples",
            "1",
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    let body = fs::read_to_string(&output).expect("read JSONL");
    assert!(body.ends_with('\n'), "JSONL must end with a newline");
    fs::remove_file(&output).ok();
}

#[test]
fn invalid_dataset_id_is_rejected_with_non_zero_exit() {
    let output = tmp("badid", "jsonl");
    let status = cli()
        .args([
            "--dataset-id",
            "", // empty string is rejected by NonEmptyString::parse
            "--minimum-examples",
            "1",
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(!status.success(), "empty --dataset-id must fail");
    fs::remove_file(&output).ok();
}

#[test]
fn zero_minimum_examples_is_rejected_with_non_zero_exit() {
    let output = tmp("zero", "jsonl");
    let status = cli()
        .args([
            "--dataset-id",
            "dataset:zero",
            "--minimum-examples",
            "0",
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(!status.success(), "--minimum-examples=0 must fail");
    fs::remove_file(&output).ok();
}

#[test]
fn deeply_nested_output_path_is_created_on_demand() {
    let parent = tmp("deep-parent", "dir");
    let output = parent.join("a/b/c/d.jsonl");
    let status = cli()
        .args([
            "--dataset-id",
            "dataset:deep",
            "--minimum-examples",
            "1",
            "--output",
            output.to_str().unwrap(),
        ])
        .status()
        .unwrap();
    assert!(status.success());
    assert!(
        output.exists(),
        "CLI must create the parent directory chain"
    );
    fs::remove_dir_all(&parent).ok();
}
