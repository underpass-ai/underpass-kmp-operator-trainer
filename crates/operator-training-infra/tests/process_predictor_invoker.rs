//! Integration tests for `ProcessPredictorInvoker`. Each test writes
//! a tiny shell script to a tempfile, makes it executable, and
//! invokes it through the adapter. Same `EXEC_LOCK` pattern as the
//! trainer invoker tests to dodge parallel `ETXTBSY` on tmpfs.

#![cfg(unix)]

use std::fs::{self, File};
use std::io::Write as _;
use std::os::unix::fs::PermissionsExt as _;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};

use operator_training_application::errors::predictor_error::PredictorError;
use operator_training_application::ports::predictor::Predictor;
use operator_training_application::ports::predictor_target::PredictorTarget;
use operator_training_domain::trainer::base_model_id::BaseModelId;
use operator_training_domain::trainer::output_directory::OutputDirectory;
use operator_training_domain::trainer::trainer_command::TrainerCommand;
use operator_training_infra::adapters::process_predictor_invoker::ProcessPredictorInvoker;

static SEQ: AtomicU64 = AtomicU64::new(1);
static EXEC_LOCK: Mutex<()> = Mutex::new(());

fn write_stub_script(label: &str, body: &str) -> std::path::PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let path = std::env::temp_dir().join(format!("operator-predict-stub-{label}-{pid}-{n}.sh"));
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

fn tmp_output_dir(label: &str) -> std::path::PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let path = std::env::temp_dir().join(format!("operator-predict-out-{label}-{pid}-{n}"));
    fs::create_dir_all(&path).expect("mkdir output_dir");
    path
}

fn target_for(script: &std::path::Path, output_dir: &std::path::Path) -> PredictorTarget {
    PredictorTarget::new(
        TrainerCommand::parse(script.to_string_lossy().to_string()).unwrap(),
        BaseModelId::parse("Qwen/Qwen2.5-0.5B-Instruct").unwrap(),
        OutputDirectory::parse("/tmp/operator-adapter").unwrap(),
        "/tmp/dataset.jsonl",
        OutputDirectory::parse(output_dir.to_string_lossy().to_string()).unwrap(),
    )
    .unwrap()
}

#[test]
fn happy_path_reads_summary_and_returns_outcome() {
    let _guard = EXEC_LOCK.lock().unwrap();
    let output_dir = tmp_output_dir("ok");
    let summary_path = output_dir.join("summary.json");
    let body = format!(
        "cat > {} <<'EOF'\n{}\nEOF\nexit 0\n",
        summary_path.to_string_lossy(),
        r#"{"predictions":3,"failures":1,"selected":4}"#
    );
    let script = write_stub_script("ok", &body);

    let invoker = ProcessPredictorInvoker::new();
    let outcome = invoker
        .predict(&target_for(&script, &output_dir))
        .expect("predict");
    assert_eq!(outcome.predictions(), 3);
    assert_eq!(outcome.failures(), 1);
    assert!(outcome.predictions_path().ends_with("predictions.jsonl"));
    assert!(outcome.summary_path().ends_with("summary.json"));

    fs::remove_file(&script).ok();
    fs::remove_dir_all(&output_dir).ok();
}

#[test]
fn non_zero_exit_yields_non_zero_outcome_error() {
    let _guard = EXEC_LOCK.lock().unwrap();
    let output_dir = tmp_output_dir("nonzero");
    let script = write_stub_script("nonzero", "exit 7\n");

    let invoker = ProcessPredictorInvoker::new();
    let err = invoker
        .predict(&target_for(&script, &output_dir))
        .expect_err("must fail");
    match err {
        PredictorError::NonZeroExit {
            exit_code: Some(7),
            adapter: "process_predictor_invoker",
            ..
        } => {}
        other => panic!("expected NonZeroExit(Some(7)), got {other:?}"),
    }

    fs::remove_file(&script).ok();
    fs::remove_dir_all(&output_dir).ok();
}

#[test]
fn unknown_command_yields_spawn_failure() {
    let _guard = EXEC_LOCK.lock().unwrap();
    let target = PredictorTarget::new(
        TrainerCommand::parse("/this/binary/does/not/exist-abc-123").unwrap(),
        BaseModelId::parse("base").unwrap(),
        OutputDirectory::parse("/tmp").unwrap(),
        "/tmp/dataset.jsonl",
        OutputDirectory::parse("/tmp").unwrap(),
    )
    .unwrap();

    let invoker = ProcessPredictorInvoker::new();
    let err = invoker.predict(&target).expect_err("must fail");
    assert!(matches!(
        err,
        PredictorError::SpawnFailure {
            adapter: "process_predictor_invoker",
            ..
        }
    ));
}

#[test]
fn missing_summary_after_exit_zero_yields_summary_unreadable() {
    let _guard = EXEC_LOCK.lock().unwrap();
    let output_dir = tmp_output_dir("nosummary");
    let script = write_stub_script("nosummary", "exit 0\n");

    let invoker = ProcessPredictorInvoker::new();
    let err = invoker
        .predict(&target_for(&script, &output_dir))
        .expect_err("must fail");
    match err {
        PredictorError::SummaryUnreadable {
            adapter: "process_predictor_invoker",
            summary_path,
            ..
        } => {
            assert!(summary_path.ends_with("summary.json"));
        }
        other => panic!("expected SummaryUnreadable, got {other:?}"),
    }

    fs::remove_file(&script).ok();
    fs::remove_dir_all(&output_dir).ok();
}

#[test]
fn nonexistent_output_directory_is_created_on_demand() {
    let _guard = EXEC_LOCK.lock().unwrap();
    // Use a deeply-nested non-existent directory; the adapter must
    // create it before spawning the child so the Python writer can
    // land its files there.
    let parent = tmp_output_dir("auto-create-parent");
    let output_dir = parent.join("a/b/c");
    let summary_path = output_dir.join("summary.json");
    let body = format!(
        "cat > {} <<'EOF'\n{}\nEOF\nexit 0\n",
        summary_path.to_string_lossy(),
        r#"{"predictions":1,"failures":0}"#
    );
    let script = write_stub_script("autocreate", &body);

    let invoker = ProcessPredictorInvoker::new();
    let outcome = invoker
        .predict(&target_for(&script, &output_dir))
        .expect("predict");
    assert_eq!(outcome.predictions(), 1);
    assert!(output_dir.exists(), "adapter must create output_dir");

    fs::remove_file(&script).ok();
    fs::remove_dir_all(&parent).ok();
}

#[test]
fn output_directory_pointing_at_existing_file_surfaces_unusable_error() {
    let _guard = EXEC_LOCK.lock().unwrap();
    // Pre-create a regular file where output_directory expects a
    // directory. create_dir_all then fails with NotADirectory.
    let parent = tmp_output_dir("collide");
    let blocker = parent.join("blocker-file");
    File::create(&blocker).expect("create blocker file");
    let invoker = ProcessPredictorInvoker::new();
    let target = PredictorTarget::new(
        TrainerCommand::parse("/bin/true").unwrap(),
        BaseModelId::parse("base").unwrap(),
        OutputDirectory::parse("/tmp").unwrap(),
        "/tmp/dataset.jsonl",
        OutputDirectory::parse(blocker.to_string_lossy().to_string()).unwrap(),
    )
    .unwrap();
    let err = invoker.predict(&target).expect_err("must fail");
    assert!(matches!(
        err,
        PredictorError::OutputDirectoryUnusable {
            adapter: "process_predictor_invoker",
            ..
        }
    ));

    fs::remove_file(&blocker).ok();
    fs::remove_dir_all(&parent).ok();
}

#[test]
fn invoker_passes_cli_args_through() {
    let _guard = EXEC_LOCK.lock().unwrap();
    let output_dir = tmp_output_dir("argv");
    let summary_path = output_dir.join("summary.json");
    let argv_log = output_dir.join("argv.log");
    let body = format!(
        "echo \"$@\" > {}\ncat > {} <<'EOF'\n{}\nEOF\nexit 0\n",
        argv_log.to_string_lossy(),
        summary_path.to_string_lossy(),
        r#"{"predictions":1,"failures":0}"#
    );
    let script = write_stub_script("argv", &body);

    let invoker = ProcessPredictorInvoker::new();
    invoker
        .predict(&target_for(&script, &output_dir))
        .expect("predict");

    let recorded = fs::read_to_string(&argv_log).expect("argv log");
    assert!(recorded.contains("--model-id"));
    assert!(recorded.contains("Qwen/Qwen2.5-0.5B-Instruct"));
    assert!(recorded.contains("--adapter"));
    assert!(recorded.contains("--dataset-jsonl"));
    assert!(recorded.contains("--output"));

    fs::remove_file(&script).ok();
    fs::remove_dir_all(&output_dir).ok();
}
