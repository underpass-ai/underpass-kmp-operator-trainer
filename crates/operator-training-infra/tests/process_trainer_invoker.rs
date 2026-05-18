//! Integration tests for `ProcessTrainerInvoker`. Each test writes a
//! tiny shell script to a tempfile, makes it executable, and invokes
//! it through the adapter. These tests are gated on a POSIX shell
//! (`/bin/sh`) and `chmod`; they are skipped on Windows.

#![cfg(unix)]

use std::fs::{self, File};
use std::io::Write as _;
use std::os::unix::fs::PermissionsExt as _;
use std::sync::atomic::{AtomicU64, Ordering};

use operator_training_application::errors::trainer_invoker_error::TrainerInvokerError;
use operator_training_application::ports::trainer_invocation_outcome::TrainerInvocationOutcome;
use operator_training_application::ports::trainer_invoker::TrainerInvoker;
use operator_training_domain::trainer::base_model_id::BaseModelId;
use operator_training_domain::trainer::output_directory::OutputDirectory;
use operator_training_domain::trainer::trainer_command::TrainerCommand;
use operator_training_domain::trainer::trainer_target::TrainerTarget;
use operator_training_infra::adapters::process_trainer_invoker::ProcessTrainerInvoker;

static SEQ: AtomicU64 = AtomicU64::new(1);

fn write_stub_script(label: &str, body: &str) -> std::path::PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let path = std::env::temp_dir().join(format!("operator-training-stub-{label}-{pid}-{n}.sh"));
    // Close the write fd explicitly before chmod + exec so Linux does
    // not return ETXTBSY when we exec the file (the kernel rejects
    // exec on files with open write descriptors).
    {
        let mut file = File::create(&path).expect("create stub");
        writeln!(file, "#!/bin/sh").unwrap();
        file.write_all(body.as_bytes()).unwrap();
        file.sync_all().unwrap();
    }
    let mut perms = fs::metadata(&path).unwrap().permissions();
    perms.set_mode(0o755);
    fs::set_permissions(&path, perms).unwrap();
    path
}

fn target_for(script: &std::path::Path) -> TrainerTarget {
    TrainerTarget::new(
        TrainerCommand::parse(script.to_string_lossy().to_string()).unwrap(),
        BaseModelId::parse("base").unwrap(),
        OutputDirectory::parse("out").unwrap(),
    )
}

#[test]
fn successful_exit_zero_yields_success_outcome() {
    let script = write_stub_script("ok", "exit 0\n");
    let target = target_for(&script);
    let invoker = ProcessTrainerInvoker::new();

    let outcome = invoker.invoke(&target).expect("invoke");
    assert!(matches!(
        outcome,
        TrainerInvocationOutcome::Success { exit_code: 0 }
    ));

    fs::remove_file(&script).ok();
}

#[test]
fn non_zero_exit_yields_failed_outcome_with_code() {
    let script = write_stub_script("fail", "exit 7\n");
    let target = target_for(&script);
    let invoker = ProcessTrainerInvoker::new();

    let outcome = invoker.invoke(&target).expect("invoke");
    assert!(matches!(
        outcome,
        TrainerInvocationOutcome::Failed { exit_code: Some(7) }
    ));

    fs::remove_file(&script).ok();
}

#[test]
fn unknown_command_yields_spawn_failure() {
    let target = TrainerTarget::new(
        TrainerCommand::parse("/this/binary/does/not/exist/anywhere-12345").unwrap(),
        BaseModelId::parse("base").unwrap(),
        OutputDirectory::parse("out").unwrap(),
    );
    let invoker = ProcessTrainerInvoker::new();

    let err = invoker.invoke(&target).expect_err("must fail");
    assert!(matches!(
        err,
        TrainerInvokerError::SpawnFailure {
            adapter: "process_trainer_invoker",
            ..
        }
    ));
}

#[test]
fn invoker_passes_base_model_and_output_directory_as_arguments() {
    // The stub records its CLI args to a sidecar file; we then assert
    // both `--base-model` and `--output-dir` made it through.
    let argv_log = std::env::temp_dir().join(format!(
        "operator-training-stub-argv-{}-{}.log",
        std::process::id(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    ));
    let script = write_stub_script(
        "argv",
        &format!("echo \"$@\" > {}\nexit 0\n", argv_log.to_string_lossy()),
    );

    let target = TrainerTarget::new(
        TrainerCommand::parse(script.to_string_lossy().to_string()).unwrap(),
        BaseModelId::parse("base-X").unwrap(),
        OutputDirectory::parse("out/dir").unwrap(),
    );
    let invoker = ProcessTrainerInvoker::new();
    let outcome = invoker.invoke(&target).expect("invoke");
    assert!(outcome.is_success());

    let recorded = fs::read_to_string(&argv_log).expect("argv log");
    assert!(recorded.contains("--base-model"));
    assert!(recorded.contains("base-X"));
    assert!(recorded.contains("--output-dir"));
    assert!(recorded.contains("out/dir"));

    fs::remove_file(&script).ok();
    fs::remove_file(&argv_log).ok();
}
