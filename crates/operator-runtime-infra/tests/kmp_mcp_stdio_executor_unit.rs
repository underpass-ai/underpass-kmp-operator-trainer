use std::fs;
use std::path::{Path, PathBuf};

use operator_runtime_application::errors::mcp_executor_error::McpExecutorError;
use operator_runtime_application::ports::mcp_executor_port::McpExecutor;
use operator_runtime_domain::session::observation::Observation;
use operator_runtime_infra::adapters::kmp_mcp_stdio_config::KmpMcpStdioConfig;
use operator_runtime_infra::adapters::kmp_mcp_stdio_executor::KmpMcpStdioExecutor;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::action::tool_call_action::ToolCallAction;
use operator_shared_domain::cursor::temporal_anchor::TemporalAnchor;
use operator_shared_domain::cursor::temporal_cursor::TemporalCursor;
use operator_shared_domain::cursor::temporal_cursor_key::TemporalCursorKey;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::tool_arguments::forward_arguments::ForwardArguments;
use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
use operator_shared_domain::tool_arguments::write_memory_arguments::WriteMemoryArguments;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::positive_count::PositiveCount;
use serde_json::Value;

#[test]
fn executor_rejects_write_tools_in_read_profile_without_spawning_process() {
    let executor = KmpMcpStdioExecutor::new(KmpMcpStdioConfig::new(
        "/path/that/does/not/exist",
        "https://kernel.example.test",
    ));
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::WriteMemory(
        WriteMemoryArguments::new("summary", "body", vec![]).unwrap(),
    )));
    let about = AboutId::parse("about:test").unwrap();

    let result = executor.execute(&action, &about);

    assert!(matches!(
        result,
        Err(McpExecutorError::WriteToolNotAllowedInReadProfile)
    ));
}

#[test]
fn executor_maps_successful_stdio_tool_response() {
    let capture_path = temp_path("stdio-success-request.json");
    let response = r#"{"jsonrpc":"2.0","id":1,"result":{"structuredContent":{"summary":"Found node","object":{"ref":"node:1","kind":"claim"},"links":{"incoming":[],"outgoing":[{"to":"node:2"}]}},"isError":false}}"#;
    let executor = KmpMcpStdioExecutor::new(shell_config(&capture_path, response));
    let about = AboutId::parse("about:test").unwrap();
    let action = inspect_action("node:1");

    let observation = executor
        .execute(&action, &about)
        .expect("stdio response maps");

    match observation {
        Observation::ToolResponse {
            outcome,
            observed_refs,
        } => {
            assert_eq!(outcome.tool().as_str(), "kernel_inspect");
            assert_eq!(
                observed_refs
                    .iter()
                    .map(MemoryRef::as_str)
                    .collect::<Vec<_>>(),
                vec!["node:1", "node:2"]
            );
        }
        other => panic!("expected tool response, got {other:?}"),
    }

    let request: Value =
        serde_json::from_str(&fs::read_to_string(capture_path).expect("request captured"))
            .expect("request JSON parses");
    assert_eq!(request["jsonrpc"], "2.0");
    assert_eq!(request["method"], "tools/call");
    assert_eq!(request["params"]["name"], "kernel_inspect");
    assert_eq!(request["params"]["arguments"]["ref"], "node:1");
}

#[test]
fn executor_maps_mcp_tool_error_to_observation() {
    let capture_path = temp_path("stdio-tool-error-request.json");
    let response = r#"{"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"not found"}],"isError":true}}"#;
    let executor = KmpMcpStdioExecutor::new(shell_config(&capture_path, response));
    let about = AboutId::parse("about:test").unwrap();

    let observation = executor
        .execute(&inspect_action("node:missing"), &about)
        .expect("tool error is an observation");

    match observation {
        Observation::ToolError { code, message } => {
            assert_eq!(code.as_str(), "mcp_tool_error");
            assert_eq!(message, "not found");
        }
        other => panic!("expected tool error observation, got {other:?}"),
    }
}

#[test]
fn executor_sends_temporal_sequence_cursor_for_forward() {
    let capture_path = temp_path("stdio-forward-request.json");
    let response = r#"{"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"not found"}],"isError":true}}"#;
    let executor = KmpMcpStdioExecutor::new(shell_config(&capture_path, response));
    let about = AboutId::parse("about:test").unwrap();

    let observation = executor
        .execute(&forward_action("seq:3"), &about)
        .expect("tool error remains an observation");

    assert!(matches!(observation, Observation::ToolError { .. }));
    let request: Value =
        serde_json::from_str(&fs::read_to_string(capture_path).expect("request captured"))
            .expect("request JSON parses");
    assert_eq!(request["params"]["name"], "kernel_forward");
    assert_eq!(request["params"]["arguments"]["about"], "about:test");
    assert_eq!(request["params"]["arguments"]["from"]["sequence"], 3);
    assert!(request["params"]["arguments"]["from"].get("key").is_none());
    assert!(
        request["params"]["arguments"]["from"]
            .get("anchor")
            .is_none()
    );
    assert_eq!(request["params"]["arguments"]["window"]["after_entries"], 2);
}

#[test]
fn executor_maps_non_zero_process_exit_to_transport_error() {
    let executor = KmpMcpStdioExecutor::new(
        KmpMcpStdioConfig::new("/bin/sh", "https://kernel.example.test")
            .with_arg("-c")
            .with_arg("echo boom >&2; exit 7"),
    );
    let about = AboutId::parse("about:test").unwrap();

    let result = executor.execute(&inspect_action("node:1"), &about);

    assert!(matches!(result, Err(McpExecutorError::Transport { .. })));
}

fn inspect_action(target: &str) -> OperatorAction {
    OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
        InspectArguments::new(MemoryRef::parse(target).unwrap()),
    )))
}

fn forward_action(anchor: &str) -> OperatorAction {
    let cursor = TemporalCursor::new(
        TemporalCursorKey::Created,
        TemporalAnchor::parse(anchor).unwrap(),
    );
    OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Forward(
        ForwardArguments::new(cursor, PositiveCount::parse(2, "window").unwrap()),
    )))
}

fn shell_config(capture_path: &Path, response: &str) -> KmpMcpStdioConfig {
    KmpMcpStdioConfig::new("/bin/sh", "https://kernel.example.test")
        .with_arg("-c")
        .with_arg("cat > \"$CAPTURE_PATH\"; printf '%s\\n' \"$MCP_RESPONSE\"")
        .with_env("CAPTURE_PATH", capture_path.display().to_string())
        .with_env("MCP_RESPONSE", response)
}

fn temp_path(name: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!("operator-runtime-{name}-{}", std::process::id()));
    let _ = fs::remove_file(&path);
    path
}
