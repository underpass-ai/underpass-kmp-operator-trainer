use std::fs;
use std::process::Command;

use serde_json::{Value, json};

#[test]
fn happy_path_with_stub_teacher_exits_zero() {
    let fixture = Fixture::new("happy");
    fixture.write_cases(&full_cases_jsonl());
    fixture.write_prompt();
    fixture.write_key();

    let output = Command::new(binary())
        .env("OPERATOR_TEACHER_CALIBRATION_STUB", "accepted")
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
}

#[test]
fn below_threshold_with_stub_teacher_exits_non_zero() {
    let fixture = Fixture::new("below-threshold");
    fixture.write_cases(&full_cases_jsonl());
    fixture.write_prompt();
    fixture.write_key();

    let output = Command::new(binary())
        .env("OPERATOR_TEACHER_CALIBRATION_STUB", "wrong")
        .args(fixture.args())
        .output()
        .unwrap();

    assert!(!output.status.success());
    let report = fs::read_to_string(fixture.output.join("report.json")).unwrap();
    assert!(report.contains("\"gate_passed\": false"));
    assert!(report.contains("\"predicted_action\""));
    assert!(report.contains("\"accepted_actions\""));
}

#[test]
fn nonexistent_cases_file_exits_with_clear_error() {
    let fixture = Fixture::new("missing-cases");
    fixture.write_prompt();
    fixture.write_key();

    let output = Command::new(binary())
        .env("OPERATOR_TEACHER_CALIBRATION_STUB", "accepted")
        .args(fixture.args())
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--cases"));
}

#[test]
fn nonexistent_prompt_file_exits_with_clear_error() {
    let fixture = Fixture::new("missing-prompt");
    fixture.write_cases(&full_cases_jsonl());
    fixture.write_key();

    let output = Command::new(binary())
        .env("OPERATOR_TEACHER_CALIBRATION_STUB", "accepted")
        .args(fixture.args())
        .output()
        .unwrap();

    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("--prompt"));
}

fn binary() -> &'static str {
    env!("CARGO_BIN_EXE_operator-teacher-calibration")
}

#[derive(Debug)]
struct Fixture {
    root: std::path::PathBuf,
    cases: std::path::PathBuf,
    prompt: std::path::PathBuf,
    key: std::path::PathBuf,
    output: std::path::PathBuf,
}

impl Fixture {
    fn new(name: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "operator-teacher-calibration-cli-{}-{name}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        Self {
            cases: root.join("cases.jsonl"),
            prompt: root.join("prompt.md"),
            key: root.join("openai.txt"),
            output: root.join("out"),
            root,
        }
    }

    fn args(&self) -> Vec<String> {
        vec![
            "--cases".to_string(),
            self.cases.display().to_string(),
            "--prompt".to_string(),
            self.prompt.display().to_string(),
            "--api-base".to_string(),
            "https://api.openai.com/v1".to_string(),
            "--api-key-file".to_string(),
            self.key.display().to_string(),
            "--model".to_string(),
            "stub-model".to_string(),
            "--temperature".to_string(),
            "0".to_string(),
            "--output".to_string(),
            self.output.display().to_string(),
            "--limit".to_string(),
            "0".to_string(),
        ]
    }

    fn write_cases(&self, jsonl: &str) {
        fs::write(&self.cases, jsonl).unwrap();
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

fn full_cases_jsonl() -> String {
    [
        case("kernel_ingest", "write", &ingest_action()),
        case(
            "kernel_wake",
            "read",
            &tool("kernel_wake", &json!({"about":"about:calibration"})),
        ),
        case(
            "kernel_ask",
            "read",
            &tool("kernel_ask", &json!({"query":"What is known now?"})),
        ),
        case(
            "kernel_near",
            "read",
            &tool(
                "kernel_near",
                &json!({"anchor": first_visible_ref(), "dimensions":["agent:operator"],"limit":3}),
            ),
        ),
        case(
            "kernel_goto",
            "read",
            &tool(
                "kernel_goto",
                &json!({"cursor":{"kind":"ref","target": first_visible_ref()}}),
            ),
        ),
        case(
            "kernel_rewind",
            "read",
            &tool("kernel_rewind", &json!({"cursor_key":"created","cursor_anchor":"seq:1","window":2})),
        ),
        case(
            "kernel_forward",
            "read",
            &tool("kernel_forward", &json!({"cursor_key":"created","cursor_anchor":"seq:1","window":2})),
        ),
        case(
            "kernel_trace",
            "read",
            &tool("kernel_trace", &json!({"from": first_visible_ref(), "to": second_visible_ref(), "page":8})),
        ),
        case(
            "kernel_inspect",
            "read",
            &tool("kernel_inspect", &json!({"target": first_visible_ref()})),
        ),
        case(
            "kernel_write_memory",
            "write",
            &tool(
                "kernel_write_memory",
                &json!({"summary":"Record calibrated memory.","body":"The calibrated write is ready.","related":[first_visible_ref()]}),
            ),
        ),
        case(
            "stop",
            "read",
            &json!({"kind":"stop","reason":"answer_ready","answer":"The evidence is sufficient.","evidence":[first_visible_ref()]}),
        ),
        case(
            "escalate",
            "read",
            &json!({"kind":"escalate","reason":"beyond_capability","target_model":"frontier-reasoner"}),
        ),
    ]
    .into_iter()
    .map(|value| value.to_string())
    .collect::<Vec<_>>()
    .join("\n")
        + "\n"
}

fn case(capability: &str, mode: &str, action: &Value) -> Value {
    json!({
        "case_id": format!("calib:{capability}"),
        "domain_theme": "technical_incident",
        "category": "happy",
        "subject": {
            "about": "about:calibration",
            "mode": mode,
            "task_family": format!("calibration.{capability}"),
            "goal": format!("Select {capability}."),
            "allowed_tools": allowed_tools(mode),
            "visible_state": visible_state(capability)
        },
        "accepted_actions": [action.clone()],
        "expected_action_rationale": "The expected action is explicit."
    })
}

fn allowed_tools(mode: &str) -> Vec<&'static str> {
    match mode {
        "write" => vec!["kernel_ingest", "kernel_write_memory"],
        _ => vec![
            "kernel_wake",
            "kernel_ask",
            "kernel_near",
            "kernel_goto",
            "kernel_rewind",
            "kernel_forward",
            "kernel_trace",
            "kernel_inspect",
        ],
    }
}

fn visible_state(capability: &str) -> Value {
    let active_cursor = matches!(capability, "kernel_rewind" | "kernel_forward")
        .then(|| json!({"kind":"temporal","key":"created","anchor":"seq:1"}));
    json!({
        "known_refs": ["node:visible", "node:other"],
        "known_dimensions": ["agent:operator"],
        "active_cursor": active_cursor,
        "budget": {"calls_remaining": 4, "tokens_remaining": 1024}
    })
}

fn first_visible_ref() -> &'static str {
    "node:other"
}

fn second_visible_ref() -> &'static str {
    "node:visible"
}

fn tool(name: &str, arguments: &Value) -> Value {
    json!({"kind":"tool_call","tool":name,"arguments":arguments.clone()})
}

fn ingest_action() -> Value {
    tool(
        "kernel_ingest",
        &json!({
            "about":"about:calibration",
            "memory":{
                "dimensions":[{"id":"agent:operator","kind":"agent","title":"Writer","metadata":{}}],
                "entries":[{
                    "id":"node:calibration:new",
                    "kind":"decision",
                    "text":"Calibrated write.",
                    "coordinates":[{
                        "dimension":"agent:operator",
                        "scope_id":"scope:writer",
                        "sequence":1,
                        "metadata":{}
                    }],
                    "metadata":{}
                }],
                "relations":[],
                "evidence":[]
            },
            "idempotency_key":"idem:calibration",
            "dry_run":true
        }),
    )
}
