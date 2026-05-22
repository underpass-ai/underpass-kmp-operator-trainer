#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::{Value, json};

static SEQ: AtomicU64 = AtomicU64::new(1);

#[test]
fn python_pipeline_accepts_all_modes_and_action_kinds() {
    let fixture = Fixture::new("full-modes");
    fixture.write_prompt();
    fixture.write_key();
    fixture.write_scenarios();

    fixture.validate_scenarios_only();
    fixture.build_realistic_corpus_with_stub_teacher();
    fixture.prepare_sft_dataset();
    fixture.validate_openai_schema();
    fixture.assert_sft_schema_preserves_contract_surface();
    fixture.validate_training_inputs();
    fixture.validate_prediction_inputs();
    fixture.assert_oracle_policy_eval_passes();
}

#[derive(Debug)]
struct Fixture {
    root: PathBuf,
    scenarios: PathBuf,
    prompt: PathBuf,
    key: PathBuf,
    corpus_out: PathBuf,
    validate_out: PathBuf,
    sft_out: PathBuf,
}

impl Fixture {
    fn new(label: &str) -> Self {
        let n = SEQ.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "operator-python-pipeline-{label}-{}-{n}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self {
            scenarios: root.join("scenarios.jsonl"),
            prompt: root.join("prompt.md"),
            key: root.join("openai.txt"),
            corpus_out: root.join("corpus"),
            validate_out: root.join("validate-only"),
            sft_out: root.join("sft"),
            root,
        }
    }

    fn write_prompt(&self) {
        fs::write(
            &self.prompt,
            "Return the exact action requested by the subject.",
        )
        .unwrap();
    }

    fn write_key(&self) {
        fs::write(&self.key, "test-key").unwrap();
    }

    fn write_scenarios(&self) {
        let mut payload = String::new();
        for scenario in scenarios() {
            payload.push_str(&serde_json::to_string(&scenario).unwrap());
            payload.push('\n');
        }
        fs::write(&self.scenarios, payload).unwrap();
    }

    fn validate_scenarios_only(&self) {
        let mut args = self.realistic_args(&self.validate_out);
        args.push("--validate-only".to_string());
        assert_success(
            &Command::new(realistic_binary())
                .args(args)
                .output()
                .unwrap(),
            "operator-realistic-corpus --validate-only",
        );
    }

    fn build_realistic_corpus_with_stub_teacher(&self) {
        let output = Command::new(realistic_binary())
            .env("OPERATOR_REALISTIC_CORPUS_STUB", "accepted")
            .args(self.realistic_args(&self.corpus_out))
            .output()
            .unwrap();
        if !output.status.success() {
            let dropped = fs::read_to_string(self.corpus_out.join("dropped.jsonl"))
                .unwrap_or_else(|err| format!("dropped unavailable: {err}"));
            panic!(
                "operator-realistic-corpus stub accepted failed\nstatus={}\nstdout={}\nstderr={}\ndropped={}",
                output.status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr),
                dropped
            );
        }
        assert!(self.corpus_out.join("trajectories.jsonl").is_file());
    }

    fn prepare_sft_dataset(&self) {
        assert_success(
            &Command::new("python3")
                .current_dir(repo_root())
                .args([
                    "scripts/operator/prepare_operator_sft_dataset.py",
                    "--trajectories",
                    self.corpus_out.join("trajectories.jsonl").to_str().unwrap(),
                    "--output",
                    self.sft_out.to_str().unwrap(),
                    "--eval-ratio",
                    "0.5",
                    "--seed",
                    "7",
                    "--force",
                ])
                .output()
                .unwrap(),
            "prepare_operator_sft_dataset.py",
        );
    }

    fn validate_openai_schema(&self) {
        assert_success(
            &Command::new("python3")
                .current_dir(repo_root())
                .args([
                    "scripts/operator/validate_openai_sft_schema.py",
                    self.sft_out.join("openai_train.jsonl").to_str().unwrap(),
                    self.sft_out.join("openai_eval.jsonl").to_str().unwrap(),
                ])
                .output()
                .unwrap(),
            "validate_openai_sft_schema.py",
        );
    }

    fn assert_sft_schema_preserves_contract_surface(&self) {
        let trajectories = read_jsonl(&self.sft_out.join("all_trajectories.jsonl"));
        assert_eq!(trajectories.len(), 12);
        assert_contains_values(
            &trajectories,
            "mode",
            &["read", "writer_pre_read", "write", "full"],
        );
        assert_contains_action_kinds(&trajectories, &["tool_call", "stop", "escalate"]);

        let writer_pre_read_tools = trajectories
            .iter()
            .filter(|row| row.get("mode").and_then(Value::as_str) == Some("writer_pre_read"))
            .filter_map(|row| row.pointer("/target_action/tool").and_then(Value::as_str))
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            writer_pre_read_tools,
            ["kernel_ask", "kernel_inspect", "kernel_near", "kernel_wake"]
                .into_iter()
                .collect()
        );

        for path in [
            self.sft_out.join("openai_train.jsonl"),
            self.sft_out.join("openai_eval.jsonl"),
        ] {
            for row in read_jsonl(&path) {
                assert!(row.get("step_id").and_then(Value::as_str).is_some());
                assert!(row.get("messages").and_then(Value::as_array).is_some());
            }
        }
    }

    fn validate_training_inputs(&self) {
        assert_success(
            &Command::new("python3")
                .current_dir(repo_root())
                .args([
                    "scripts/operator/train_operator_sft_lora.py",
                    "--train-jsonl",
                    self.sft_out.join("openai_train.jsonl").to_str().unwrap(),
                    "--eval-jsonl",
                    self.sft_out.join("openai_eval.jsonl").to_str().unwrap(),
                    "--output-dir",
                    self.root.join("train-validate-only").to_str().unwrap(),
                    "--validate-only",
                ])
                .output()
                .unwrap(),
            "train_operator_sft_lora.py --validate-only",
        );
    }

    fn validate_prediction_inputs(&self) {
        assert_success(
            &Command::new("python3")
                .current_dir(repo_root())
                .args([
                    "scripts/operator/predict_operator_sft.py",
                    "--dataset-jsonl",
                    self.sft_out.join("eval.jsonl").to_str().unwrap(),
                    "--model-id",
                    "pipeline-full-modes-smoke",
                    "--output",
                    self.root.join("predict-validate-only").to_str().unwrap(),
                    "--resolve-prepared-payloads",
                    "--validate-only",
                ])
                .output()
                .unwrap(),
            "predict_operator_sft.py --validate-only",
        );
    }

    fn assert_oracle_policy_eval_passes(&self) {
        let predictions = self.root.join("oracle_predictions.jsonl");
        write_oracle_predictions(&self.sft_out.join("eval_trajectories.jsonl"), &predictions);
        assert_success(
            &Command::new(env!("CARGO"))
                .current_dir(repo_root())
                .args([
                    "run",
                    "--quiet",
                    "-p",
                    "operator-evaluation-cli",
                    "--bin",
                    "operator-policy-eval",
                    "--",
                    "--predictions",
                    predictions.to_str().unwrap(),
                    "--ground-truth",
                    self.sft_out
                        .join("eval_trajectories.jsonl")
                        .to_str()
                        .unwrap(),
                    "--min-pass-rate",
                    "1.0",
                ])
                .output()
                .unwrap(),
            "operator-policy-eval oracle",
        );
    }

    fn realistic_args(&self, output: &Path) -> Vec<String> {
        vec![
            "--scenarios".to_string(),
            self.scenarios.display().to_string(),
            "--output".to_string(),
            output.display().to_string(),
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
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn realistic_binary() -> &'static str {
    env!("CARGO_BIN_EXE_operator-realistic-corpus")
}

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}

fn assert_success(output: &Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} failed\nstatus={}\nstdout={}\nstderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn read_jsonl(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn write_oracle_predictions(eval_trajectories: &Path, predictions: &Path) {
    let mut payload = String::new();
    for row in read_jsonl(eval_trajectories) {
        payload.push_str(
            &serde_json::to_string(&json!({
                "step_id": row["step_id"],
                "action": row["target_action"],
            }))
            .unwrap(),
        );
        payload.push('\n');
    }
    fs::write(predictions, payload).unwrap();
}

fn assert_contains_values(rows: &[Value], field: &str, expected: &[&str]) {
    let actual = rows
        .iter()
        .filter_map(|row| row.get(field).and_then(Value::as_str))
        .collect::<std::collections::BTreeSet<_>>();
    for value in expected {
        assert!(
            actual.contains(value),
            "missing {field}={value}; got {actual:?}"
        );
    }
}

fn assert_contains_action_kinds(rows: &[Value], expected: &[&str]) {
    let actual = rows
        .iter()
        .filter_map(|row| row.pointer("/target_action/kind").and_then(Value::as_str))
        .collect::<std::collections::BTreeSet<_>>();
    for value in expected {
        assert!(
            actual.contains(value),
            "missing action kind {value}; got {actual:?}"
        );
    }
}

fn scenarios() -> Vec<Value> {
    vec![
        scenario(
            "scenario:pipeline:read:wake",
            "kernel_wake",
            "read",
            "realistic.kernel_wake.read",
            None,
        ),
        scenario(
            "scenario:pipeline:read:stop",
            "stop",
            "read",
            "realistic.stop.answer",
            None,
        ),
        scenario(
            "scenario:pipeline:read:escalate",
            "escalate",
            "read",
            "realistic.escalate.beyond",
            None,
        ),
        scenario(
            "scenario:pipeline:wpr:wake",
            "kernel_wake",
            "writer_pre_read",
            "realistic.kernel_wake.writer_pre_read",
            None,
        ),
        scenario(
            "scenario:pipeline:wpr:ask",
            "kernel_ask",
            "writer_pre_read",
            "realistic.writer_pre_read.ask",
            Some(tool_action(
                "kernel_ask",
                json!({"query": "Which evidence ref is canonical?"}),
            )),
        ),
        scenario(
            "scenario:pipeline:wpr:near",
            "kernel_near",
            "writer_pre_read",
            "realistic.writer_pre_read.near",
            Some(tool_action(
                "kernel_near",
                json!({"anchor": "node:visible", "dimensions": ["agent:triage"], "limit": 8}),
            )),
        ),
        scenario(
            "scenario:pipeline:wpr:inspect",
            "kernel_inspect",
            "writer_pre_read",
            "realistic.writer_pre_read.inspect",
            None,
        ),
        scenario(
            "scenario:pipeline:full:inspect",
            "kernel_inspect",
            "full",
            "realistic.full.inspect",
            None,
        ),
        scenario(
            "scenario:pipeline:full:stop",
            "stop",
            "full",
            "realistic.full.stop",
            None,
        ),
        scenario(
            "scenario:pipeline:full:escalate",
            "escalate",
            "full",
            "realistic.full.escalate",
            None,
        ),
        scenario(
            "scenario:pipeline:write:write-memory",
            "kernel_write_memory",
            "write",
            "realistic.write.write_memory",
            Some(tool_action(
                "kernel_write_memory",
                json!({
                    "summary": "Record selected relation.",
                    "body": "Visible evidence supports the selected relation.",
                    "related": ["node:visible"]
                }),
            )),
        ),
        scenario(
            "scenario:pipeline:write:ingest",
            "kernel_ingest",
            "write",
            "realistic.write.ingest",
            Some(tool_action("kernel_ingest", ingest_arguments())),
        ),
    ]
}

fn scenario(
    scenario_id: &str,
    target: &str,
    mode: &str,
    task_family: &str,
    prepared_action: Option<Value>,
) -> Value {
    let about = format!("about:{scenario_id}");
    let mut subject = json!({
        "about": about,
        "mode": mode,
        "task_family": task_family,
        "goal": format!("Pipeline drift fixture for {scenario_id}."),
        "allowed_tools": allowed_tools(mode),
        "visible_state": {
            "known_refs": ["node:visible", "node:target"],
            "known_dimensions": ["agent:triage"],
            "active_cursor": null,
            "budget": {"calls_remaining": 4, "tokens_remaining": 1200}
        }
    });
    if let Some(action) = prepared_action {
        subject["prepared_action"] = action;
    }
    json!({
        "scenario_id": scenario_id,
        "target": target,
        "subject": subject
    })
}

fn allowed_tools(mode: &str) -> Vec<&'static str> {
    match mode {
        "read" => vec![
            "kernel_wake",
            "kernel_ask",
            "kernel_near",
            "kernel_goto",
            "kernel_rewind",
            "kernel_forward",
            "kernel_trace",
            "kernel_inspect",
        ],
        "writer_pre_read" => vec!["kernel_wake", "kernel_ask", "kernel_near", "kernel_inspect"],
        "write" => vec!["kernel_ingest", "kernel_write_memory"],
        "full" => vec![
            "kernel_ingest",
            "kernel_wake",
            "kernel_ask",
            "kernel_near",
            "kernel_goto",
            "kernel_rewind",
            "kernel_forward",
            "kernel_trace",
            "kernel_inspect",
            "kernel_write_memory",
        ],
        other => panic!("unsupported fixture mode {other}"),
    }
}

fn tool_action(tool: &str, arguments: Value) -> Value {
    let mut action = serde_json::Map::new();
    action.insert("kind".to_string(), Value::String("tool_call".to_string()));
    action.insert("tool".to_string(), Value::String(tool.to_string()));
    action.insert("arguments".to_string(), arguments);
    Value::Object(action)
}

fn ingest_arguments() -> Value {
    json!({
        "about": "about:scenario:pipeline:write:ingest",
        "memory": {
            "dimensions": [{"id": "agent:triage", "kind": "agent"}],
            "entries": [{
                "id": "entry:1",
                "kind": "decision",
                "text": "Record the selected triage decision.",
                "coordinates": [{
                    "dimension": "agent:triage",
                    "scope_id": "scope:triage",
                    "sequence": 1
                }]
            }],
            "relations": [],
            "evidence": []
        },
        "provenance": {
            "source_kind": "agent",
            "source_agent": "operator-pipeline-smoke",
            "observed_at": "2026-05-22T00:00:00Z",
            "correlation_id": "corr:pipeline",
            "causation_id": "cause:pipeline"
        },
        "idempotency_key": "pipeline-ingest-1",
        "dry_run": true
    })
}
