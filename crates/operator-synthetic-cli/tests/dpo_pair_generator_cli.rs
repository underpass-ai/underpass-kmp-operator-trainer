use std::fs;
use std::process::Command;

use serde_json::Value;

#[test]
fn dpo_pair_generator_emits_valid_pairs_and_skip_report() {
    let fixture = Fixture::new("smoke");
    fixture.write_scenarios();
    fixture.write_train_jsonl();

    let output = Command::new(binary())
        .args([
            "--train-jsonl",
            fixture.train_jsonl.to_str().unwrap(),
            "--scenarios-jsonl",
            fixture.scenarios_jsonl.to_str().unwrap(),
            "--output",
            fixture.pairs_jsonl.to_str().unwrap(),
            "--max-per-row",
            "6",
            "--force",
        ])
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "stdout={}\nstderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let pairs = read_jsonl(&fixture.pairs_jsonl);
    assert!(!pairs.is_empty());
    assert!(fixture.root.join("summary.json").is_file());
    assert!(fixture.root.join("skipped_perturbations.jsonl").is_file());
    for pair in pairs {
        assert_ne!(pair["chosen"], pair["rejected"]);
        assert!(pair["perturbation"]["name"].as_str().is_some());
        assert!(pair["rejected_violation_codes"].as_array().is_some());
    }
}

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_operator-dpo-pair-generator")
}

#[derive(Debug)]
struct Fixture {
    root: std::path::PathBuf,
    scenarios_jsonl: std::path::PathBuf,
    train_jsonl: std::path::PathBuf,
    pairs_jsonl: std::path::PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "operator-dpo-pair-generator-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self {
            scenarios_jsonl: root.join("scenarios.jsonl"),
            train_jsonl: root.join("openai_train.jsonl"),
            pairs_jsonl: root.join("pairs.jsonl"),
            root,
        }
    }

    fn write_scenarios(&self) {
        fs::write(&self.scenarios_jsonl, scenario_line()).unwrap();
    }

    fn write_train_jsonl(&self) {
        let row = serde_json::json!({
            "step_id": "scenario:dpo-write:0001:step:0001",
            "messages": [
                {"role": "system", "content": "Return exactly one JSON action."},
                {"role": "user", "content": "{\"about\":\"about:dpo\",\"mode\":\"write\",\"allowed_tools\":[\"kernel_write_memory\",\"kernel_ingest\"],\"visible_state\":{\"known_refs\":[\"about:dpo:node:a\",\"about:dpo:node:b\",\"about:dpo:node:c\"],\"known_dimensions\":[],\"budget\":{\"calls_remaining\":4,\"tokens_remaining\":1000}}}"},
                {"role": "assistant", "content": "{\"action\":{\"kind\":\"tool_call\",\"tool\":\"kernel_write_memory\",\"arguments\":{\"summary\":\"Record DPO fixture.\",\"body\":\"Body references visible nodes.\",\"related\":[\"about:dpo:node:a\",\"about:dpo:node:b\"]}}}"}
            ]
        });
        fs::write(
            &self.train_jsonl,
            serde_json::to_string(&row).unwrap() + "\n",
        )
        .unwrap();
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

fn scenario_line() -> String {
    r#"{"scenario_id":"scenario:dpo-write:0001","target":"kernel_write_memory","subject":{"about":"about:dpo","mode":"write","task_family":"realistic.kernel_write_memory.dpo_fixture","goal":"Execute the prepared write.","allowed_tools":["kernel_write_memory","kernel_ingest"],"visible_state":{"known_refs":["about:dpo:node:a","about:dpo:node:b","about:dpo:node:c"],"known_dimensions":[],"budget":{"calls_remaining":4,"tokens_remaining":1000}}}}"#.to_string()
}

fn read_jsonl(path: &std::path::Path) -> Vec<Value> {
    fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
