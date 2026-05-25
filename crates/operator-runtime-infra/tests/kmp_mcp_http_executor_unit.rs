use operator_runtime_application::errors::mcp_executor_error::McpExecutorError;
use operator_runtime_application::ports::mcp_executor_port::McpExecutor;
use operator_runtime_infra::adapters::kmp_mcp_http_executor::KmpMcpHttpExecutor;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::action::tool_call_action::ToolCallAction;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
use operator_shared_domain::tool_arguments::write_memory_arguments::WriteMemoryArguments;

#[test]
fn executor_rejects_write_tools_in_read_profile_without_network_call() {
    let executor = KmpMcpHttpExecutor::new("http://127.0.0.1:1/mcp").expect("client builds");
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
