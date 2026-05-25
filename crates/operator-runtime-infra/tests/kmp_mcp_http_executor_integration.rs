use operator_runtime_application::ports::mcp_executor_port::McpExecutor;
use operator_runtime_domain::session::observation::Observation;
use operator_runtime_infra::adapters::kmp_mcp_http_executor::KmpMcpHttpExecutor;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::action::tool_call_action::ToolCallAction;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;

#[test]
#[ignore = "requires live KMP MCP JSON-RPC endpoint; set KMP_TEST_ENDPOINT"]
fn executes_kernel_inspect_against_live_kernel() {
    let Ok(endpoint) = std::env::var("KMP_TEST_ENDPOINT") else {
        eprintln!("KMP_TEST_ENDPOINT not set; skipped");
        return;
    };
    let executor = KmpMcpHttpExecutor::new(endpoint).expect("executor builds");
    let action = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
        InspectArguments::new(MemoryRef::parse("node:1").unwrap()),
    )));
    let about = AboutId::parse("about:test").unwrap();

    let result = executor.execute(&action, &about);

    assert!(matches!(
        result,
        Ok(Observation::ToolResponse { .. } | Observation::ToolError { .. })
    ));
}
