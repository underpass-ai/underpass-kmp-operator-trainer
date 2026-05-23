//! Filesystem round-trip tests for `JsonlPredictionsReader` against
//! tempfile JSONL inputs that mimic `predict_operator_sft.py`'s
//! output.

use std::fs;
use std::io::Write as _;
use std::sync::atomic::{AtomicU64, Ordering};

use operator_evaluation_infra::adapters::jsonl_predictions_reader::JsonlPredictionsReader;
use operator_evaluation_infra::errors::predictions_read_error::PredictionsReadError;
use operator_shared_domain::ids::step_id::StepId;

static SEQ: AtomicU64 = AtomicU64::new(1);

fn tmp_jsonl(label: &str, body: &str) -> std::path::PathBuf {
    let n = SEQ.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    let path = std::env::temp_dir().join(format!("operator-eval-pred-{label}-{pid}-{n}.jsonl"));
    let mut file = fs::File::create(&path).expect("create tempfile");
    file.write_all(body.as_bytes()).expect("write body");
    file.sync_all().expect("sync");
    path
}

#[test]
fn parses_one_inspect_prediction() {
    let body = r#"{"step_id":"step:1","action":{"kind":"tool_call","tool":"kernel_inspect","arguments":{"target":"node:1"}}}
"#;
    let path = tmp_jsonl("inspect", body);
    let reader = JsonlPredictionsReader::new(&path);
    let outcome = reader.read().expect("read");

    assert_eq!(outcome.parsed().len(), 1);
    assert!(outcome.shape_violations().is_empty());
    assert_eq!(outcome.parsed()[0].step_id().as_str(), "step:1");
    fs::remove_file(&path).ok();
}

#[test]
fn skips_empty_and_whitespace_lines() {
    let body = "\n   \n{\"step_id\":\"step:1\",\"action\":{\"kind\":\"stop\",\"reason\":\"answer_ready\",\"final_refs\":[]}}\n\n";
    let path = tmp_jsonl("blank", body.to_string().as_str());
    let reader = JsonlPredictionsReader::new(&path);
    let outcome = reader.read().expect("read");

    assert_eq!(outcome.parsed().len(), 1);
    assert!(outcome.shape_violations().is_empty());
    assert_eq!(outcome.parsed()[0].step_id().as_str(), "step:1");
    fs::remove_file(&path).ok();
}

#[test]
fn returns_source_unavailable_when_file_missing() {
    let reader = JsonlPredictionsReader::new("/no/such/predictions.jsonl");
    let err = reader.read().expect_err("must fail");
    assert!(matches!(
        err,
        PredictionsReadError::SourceUnavailable {
            adapter: "jsonl_predictions_reader",
            ..
        }
    ));
}

#[test]
fn invalid_json_surfaces_invalid_row_with_line_number() {
    let body = "this-is-not-json\n";
    let path = tmp_jsonl("badjson", body);
    let reader = JsonlPredictionsReader::new(&path);
    let err = reader.read().expect_err("must fail");
    match err {
        PredictionsReadError::InvalidRow { adapter, line, .. } => {
            assert_eq!(adapter, "jsonl_predictions_reader");
            assert_eq!(line, 1);
        }
        other => panic!("expected InvalidRow, got {other:?}"),
    }
    fs::remove_file(&path).ok();
}

#[test]
fn missing_step_id_surfaces_shape_violation() {
    // Empty step_id triggers a domain validation error inside StepId::parse.
    let body = r#"{"step_id":"","action":{"kind":"stop","reason":"answer_ready","final_refs":[]}}
"#;
    let path = tmp_jsonl("noid", body);
    let reader = JsonlPredictionsReader::new(&path);
    let err = reader.read().expect_err("must fail");
    match err {
        PredictionsReadError::ShapeViolation {
            adapter,
            line,
            message,
        } => {
            assert_eq!(adapter, "jsonl_predictions_reader");
            assert_eq!(line, 1);
            assert!(message.contains("step_id"));
        }
        other => panic!("expected ShapeViolation, got {other:?}"),
    }
    fs::remove_file(&path).ok();
}

#[test]
fn second_line_failure_reports_correct_line_number() {
    let body = "{\"step_id\":\"step:1\",\"action\":{\"kind\":\"stop\",\"reason\":\"answer_ready\",\"final_refs\":[]}}\nnot-json\n";
    let path = tmp_jsonl("line2", body);
    let reader = JsonlPredictionsReader::new(&path);
    let err = reader.read().expect_err("must fail");
    match err {
        PredictionsReadError::InvalidRow { line, .. } => {
            assert_eq!(line, 2);
        }
        other => panic!("expected InvalidRow, got {other:?}"),
    }
    fs::remove_file(&path).ok();
}

#[test]
fn reads_multiple_predictions_in_order() {
    let body = r#"{"step_id":"step:1","action":{"kind":"stop","reason":"answer_ready","final_refs":[]}}
{"step_id":"step:2","action":{"kind":"tool_call","tool":"kernel_inspect","arguments":{"target":"node:2"}}}
{"step_id":"step:3","action":{"kind":"stop","reason":"answer_ready","final_refs":[]}}
"#;
    let path = tmp_jsonl("multi", body);
    let reader = JsonlPredictionsReader::new(&path);
    let outcome = reader.read().expect("read");
    let ids: Vec<&str> = outcome
        .parsed()
        .iter()
        .map(|p| p.step_id().as_str())
        .collect();
    assert_eq!(ids, vec!["step:1", "step:2", "step:3"]);
    assert!(outcome.shape_violations().is_empty());
    fs::remove_file(&path).ok();
}

#[test]
fn shape_violation_does_not_abort_read_when_recoverable() {
    let body = r#"{"step_id":"step:1","action":{"kind":"tool_call","tool":"kernel_inspect","arguments":{"target":"node:1"}}}
{"step_id":"step:2","action":{"kind":"tool_call","tool":"kernel_ingest","arguments":{"about":"about:1","memory":{"dimensions":[],"entries":[],"relations":[],"evidence":[]},"provenance":{"source_kind":"agent","source_agent":"agent:1","observed_at":"...","correlation_id":"corr:1","causation_id":"cause:1"},"idempotency_key":"idem:1","dry_run":true}}}
{"step_id":"step:3","action":{"kind":"stop","reason":"answer_ready","final_refs":[]}}
"#;
    let path = tmp_jsonl("recoverable-shape", body);
    let reader = JsonlPredictionsReader::new(&path);
    let outcome = reader.read().expect("recoverable shape violation");

    let parsed_ids: Vec<&str> = outcome
        .parsed()
        .iter()
        .map(|p| p.step_id().as_str())
        .collect();
    assert_eq!(parsed_ids, vec!["step:1", "step:3"]);
    assert_eq!(outcome.shape_violations().len(), 1);
    let violation = &outcome.shape_violations()[0];
    assert_eq!(violation.line(), 2);
    assert_eq!(violation.step_id().map(StepId::as_str), Some("step:2"));
    assert!(violation.message().contains("action:"));
    fs::remove_file(&path).ok();
}
