use std::fs;
use std::process::Command;

#[test]
fn happy_path_with_stub_teacher_exits_zero() {
    let fixture = Fixture::new("happy");
    fixture.write_scenarios();
    fixture.write_prompt();
    fixture.write_key();

    let output = Command::new(binary())
        .env("OPERATOR_REALISTIC_CORPUS_STUB", "accepted")
        .args(fixture.args())
        .output()
        .unwrap();

    let report = fs::read_to_string(fixture.output.join("report.json")).unwrap();
    assert!(
        output.status.success(),
        "stdout={}\nstderr={}\nreport={report}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(report.contains("\"gate_passed\": true"));
    assert!(
        fs::read_to_string(fixture.output.join("trajectories.jsonl"))
            .unwrap()
            .contains("\"target_action\"")
    );
    assert!(fixture.output.join("dropped.jsonl").is_file());
    assert!(!fixture.output.join("trajectories.partial.jsonl").exists());
    assert!(!fixture.output.join("dropped.partial.jsonl").exists());
    assert!(String::from_utf8_lossy(&output.stderr).contains("realistic_corpus.accepted"));
}

#[test]
fn gate_failed_with_stub_teacher_exits_non_zero() {
    let fixture = Fixture::new("gate-failed");
    fixture.write_scenarios();
    fixture.write_prompt();
    fixture.write_key();

    let output = Command::new(binary())
        .env("OPERATOR_REALISTIC_CORPUS_STUB", "wrong")
        .args(fixture.args())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let report = fs::read_to_string(fixture.output.join("report.json")).unwrap();
    assert!(report.contains("\"gate_passed\": false"));
    assert!(report.contains("\"target_mismatch\""));
    assert!(fixture.output.join("trajectories.jsonl").is_file());
    assert!(fixture.output.join("dropped.jsonl").is_file());
    assert!(!fixture.output.join("trajectories.partial.jsonl").exists());
    assert!(!fixture.output.join("dropped.partial.jsonl").exists());
}

#[test]
fn nonexistent_scenarios_file_exits_with_clear_error() {
    let fixture = Fixture::new("missing-scenarios");
    fixture.write_prompt();
    fixture.write_key();

    let output = Command::new(binary())
        .env("OPERATOR_REALISTIC_CORPUS_STUB", "accepted")
        .args(fixture.args())
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--scenarios"));
}

#[test]
fn validate_only_parses_scenarios_without_teacher_call() {
    let fixture = Fixture::new("validate-only");
    fixture.write_scenarios();
    fixture.write_prompt();
    fixture.write_key();
    let mut args = fixture.args();
    args.push("--validate-only".to_string());

    let output = Command::new(binary()).args(args).output().unwrap();

    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("\"scenarios_count\":1"));
    assert!(!fixture.output.join("report.json").exists());
}

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_operator-realistic-corpus")
}

#[derive(Debug)]
struct Fixture {
    root: std::path::PathBuf,
    scenarios: std::path::PathBuf,
    prompt: std::path::PathBuf,
    key: std::path::PathBuf,
    output: std::path::PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "operator-realistic-corpus-cli-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self {
            scenarios: root.join("scenarios.jsonl"),
            prompt: root.join("prompt.md"),
            key: root.join("openai.txt"),
            output: root.join("out"),
            root,
        }
    }

    fn args(&self) -> Vec<String> {
        vec![
            "--scenarios".to_string(),
            self.scenarios.display().to_string(),
            "--output".to_string(),
            self.output.display().to_string(),
            "--api-base".to_string(),
            "https://api.openai.com/v1".to_string(),
            "--api-key-file".to_string(),
            self.key.display().to_string(),
            "--prompt".to_string(),
            self.prompt.display().to_string(),
            "--model".to_string(),
            "stub-model".to_string(),
            "--temperature".to_string(),
            "0".to_string(),
            "--max-drop-rate".to_string(),
            "0.05".to_string(),
            "--limit".to_string(),
            "0".to_string(),
        ]
    }

    fn write_scenarios(&self) {
        fs::write(&self.scenarios, scenario_line()).unwrap();
    }

    fn write_prompt(&self) {
        fs::write(&self.prompt, "Choose one action.").unwrap();
    }

    fn write_key(&self) {
        fs::write(&self.key, "test-key").unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn scenario_line() -> String {
    r#"{"scenario_id":"scenario:inspect","target":"kernel_inspect","subject":{"about":"about:incident","mode":"read","task_family":"realistic.kernel_inspect","goal":"Inspect visible evidence.","allowed_tools":["kernel_wake","kernel_ask","kernel_near","kernel_goto","kernel_rewind","kernel_forward","kernel_trace","kernel_inspect"],"visible_state":{"known_refs":["node:target"],"known_dimensions":[],"budget":{"calls_remaining":3,"tokens_remaining":1024}}}}"#.to_string()
}
