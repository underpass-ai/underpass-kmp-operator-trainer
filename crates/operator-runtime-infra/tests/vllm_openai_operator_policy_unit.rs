use operator_runtime_infra::adapters::vllm_openai_operator_policy::VllmOpenAiOperatorPolicy;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::mode::allowed_tools::AllowedTools;
use operator_shared_domain::mode::operator_mode::OperatorMode;
use operator_shared_domain::value_objects::task_family::TaskFamily;
use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
use operator_shared_domain::visible_state::visible_state::VisibleState;
use operator_synthetic_domain::calibration::calibration_subject::CalibrationSubject;

fn test_subject_inspect() -> CalibrationSubject {
    CalibrationSubject::new(
        AboutId::parse("about:test").unwrap(),
        OperatorMode::Read,
        TaskFamily::parse("runtime.single_step").unwrap(),
        TrajectoryGoal::parse("Inspect node one.").unwrap(),
        AllowedTools::for_mode(OperatorMode::Read),
        VisibleState::assemble([], [], None, BudgetSnapshot::bounded(1, 4096)),
        None,
    )
    .unwrap()
}

#[test]
fn build_request_body_includes_strict_json_schema() {
    let policy = VllmOpenAiOperatorPolicy::for_testing();
    let body = policy
        .build_request_body(&test_subject_inspect())
        .expect("request body builds");

    assert_eq!(body["model"], "operator-v8.1.2");
    assert_eq!(body["response_format"]["type"], "json_schema");
    assert_eq!(
        body["response_format"]["json_schema"]["name"],
        "VllmOperatorAction"
    );
    assert_eq!(body["response_format"]["json_schema"]["strict"], true);
    assert_eq!(body["temperature"], 0.0);
    assert_eq!(body["max_tokens"], 1024);
}

#[test]
fn parse_response_to_action_handles_canonical_wire_format() {
    let fixture_response = r#"{"choices":[{"message":{"content":"{\"action\":{\"kind\":\"tool_call\",\"tool\":\"kernel_inspect\",\"arguments\":{\"target\":\"node:1\"}}}"}}]}"#;
    let policy = VllmOpenAiOperatorPolicy::for_testing();

    let action = policy
        .parse_response_str(fixture_response)
        .expect("canonical response parses");

    assert_eq!(action.tool().expect("tool call").as_str(), "kernel_inspect");
}

#[test]
fn parse_response_rejects_cross_kind_fields() {
    let fixture_invalid = r#"{"choices":[{"message":{"content":"{\"action\":{\"kind\":\"stop\",\"reason\":\"answer_ready\",\"answer\":null,\"evidence\":[],\"tool\":\"kernel_inspect\"}}"}}]}"#;
    let policy = VllmOpenAiOperatorPolicy::for_testing();

    assert!(policy.parse_response_str(fixture_invalid).is_err());
}

#[test]
fn parse_response_rejects_non_stop_finish_reason_before_content_parse() {
    let fixture_invalid = r#"{"choices":[{"finish_reason":"length","message":{"content":"{\"action\":{\"kind\":\"tool_call\",\"tool\":\"kernel_inspect\",\"arguments\":{\"target\":\"node:1\"}}}"}}]}"#;
    let policy = VllmOpenAiOperatorPolicy::for_testing();

    let error = policy
        .parse_response_str(fixture_invalid)
        .expect_err("length finish reason is not accepted");

    assert!(error.to_string().contains("finish_reason=length"));
}

#[test]
fn default_system_prompt_carries_the_full_mcp_api_schema() {
    // Directive B parity gate: the runtime must serve the model the same
    // full-schema prompt it was trained on. A schemaless default (the old
    // 163-char string) is what produced the v8.1.8 read-nav "cliff".
    let policy = VllmOpenAiOperatorPolicy::for_testing();
    let body = policy
        .build_request_body(&test_subject_inspect())
        .expect("request body builds");
    let system = body["messages"][0]["content"]
        .as_str()
        .expect("system message is a string");

    assert!(
        system.contains("Allowed action shapes"),
        "default system prompt must embed the MCP/API tool schema"
    );
    for tool in [
        "kernel_wake",
        "kernel_ask",
        "kernel_near",
        "kernel_goto",
        "kernel_rewind",
        "kernel_forward",
        "kernel_trace",
        "kernel_inspect",
        "kernel_write_memory",
        "kernel_ingest",
    ] {
        assert!(
            system.contains(tool),
            "default system prompt must document tool {tool}"
        );
    }
    // The full-schema prompt is ~3.7k chars; the deprecated schemaless default
    // was 163 chars. Guard against a regression to a short prompt.
    assert!(
        system.len() > 3000,
        "default system prompt is too short to carry the schema: {} chars",
        system.len()
    );
}

#[test]
fn empty_system_prompt_does_not_replace_default_prompt() {
    let policy = VllmOpenAiOperatorPolicy::for_testing();
    let default_body = policy
        .build_request_body(&test_subject_inspect())
        .expect("default body builds");
    let blank_body = VllmOpenAiOperatorPolicy::for_testing()
        .with_system_prompt("  ")
        .build_request_body(&test_subject_inspect())
        .expect("blank override body builds");

    assert_eq!(blank_body["messages"][0], default_body["messages"][0]);
}

#[test]
fn parse_response_rejects_missing_tool_arguments() {
    let fixture_invalid = r#"{"choices":[{"message":{"content":"{\"action\":{\"kind\":\"tool_call\",\"tool\":\"kernel_inspect\"}}"}}]}"#;
    let policy = VllmOpenAiOperatorPolicy::for_testing();

    let error = policy
        .parse_response_str(fixture_invalid)
        .expect_err("missing arguments rejected");

    assert!(error.to_string().contains("arguments"));
}

#[test]
fn parse_response_rejects_null_tool_arguments() {
    let fixture_invalid = r#"{"choices":[{"message":{"content":"{\"action\":{\"kind\":\"tool_call\",\"tool\":\"kernel_inspect\",\"arguments\":null}}"}}]}"#;
    let policy = VllmOpenAiOperatorPolicy::for_testing();

    let error = policy
        .parse_response_str(fixture_invalid)
        .expect_err("null arguments rejected");

    assert!(error.to_string().contains("arguments"));
}

#[test]
fn parse_response_rejects_extra_envelope_fields() {
    let fixture_invalid = r#"{"choices":[{"message":{"content":"{\"action\":{\"kind\":\"tool_call\",\"tool\":\"kernel_inspect\",\"arguments\":{\"target\":\"node:1\"}},\"debug\":true}"}}]}"#;
    let policy = VllmOpenAiOperatorPolicy::for_testing();

    let error = policy
        .parse_response_str(fixture_invalid)
        .expect_err("extra envelope field rejected");

    assert!(error.to_string().contains("envelope field"));
}

#[test]
fn parse_response_rejects_extra_argument_fields() {
    let fixture_invalid = r#"{"choices":[{"message":{"content":"{\"action\":{\"kind\":\"tool_call\",\"tool\":\"kernel_inspect\",\"arguments\":{\"target\":\"node:1\",\"debug\":true}}}"}}]}"#;
    let policy = VllmOpenAiOperatorPolicy::for_testing();

    let error = policy
        .parse_response_str(fixture_invalid)
        .expect_err("extra argument field rejected");

    assert!(error.to_string().contains("debug"));
}

#[test]
fn parse_response_rejects_invalid_wire_shape() {
    let fixture_invalid = r#"{"choices":[{"message":{"content":"not json"}}]}"#;
    let policy = VllmOpenAiOperatorPolicy::for_testing();

    assert!(policy.parse_response_str(fixture_invalid).is_err());
}
