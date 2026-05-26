use std::io::{Read, Write};
use std::net::TcpListener;
use std::thread;

use operator_runtime_application::errors::mcp_executor_error::McpExecutorError;
use operator_runtime_application::ports::mcp_executor_port::McpExecutor;
use operator_runtime_domain::session::observation::Observation;
use operator_runtime_infra::adapters::kmp_mcp_http_executor::KmpMcpHttpExecutor;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::action::tool_call_action::ToolCallAction;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::tool_arguments::near_arguments::NearArguments;
use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
use operator_shared_domain::tool_arguments::write_memory_arguments::WriteMemoryArguments;
use operator_shared_domain::value_objects::dimension_ref::DimensionRef;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::positive_count::PositiveCount;
use serde_json::Value;

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

#[test]
fn executor_sends_runtime_about_dimensions_and_limit_over_http() {
    let response = r#"{"jsonrpc":"2.0","id":1,"result":{"content":[{"type":"text","text":"not found"}],"isError":true}}"#;
    let (endpoint, handle) = serve_one_json_rpc_response(response);
    let executor = KmpMcpHttpExecutor::new(endpoint).expect("client builds");
    let about = AboutId::parse("about:test").unwrap();

    let observation = executor
        .execute(
            &near_action("node:1", &["temporal", "semantic"], Some(4)),
            &about,
        )
        .expect("tool error is an observation");

    assert!(matches!(observation, Observation::ToolError { .. }));
    let request_body = handle.join().expect("server captures request");
    let request: Value = serde_json::from_str(&request_body).expect("request JSON parses");
    assert_eq!(request["params"]["name"], "kernel_near");
    assert_eq!(request["params"]["arguments"]["about"], "about:test");
    assert_eq!(request["params"]["arguments"]["around"]["ref"], "node:1");
    assert_eq!(request["params"]["arguments"]["dimensions"]["mode"], "only");
    assert_eq!(
        request["params"]["arguments"]["dimensions"]["include"],
        serde_json::json!(["temporal", "semantic"])
    );
    assert_eq!(request["params"]["arguments"]["limit"]["entries"], 4);
}

fn near_action(anchor: &str, dimensions: &[&str], limit: Option<usize>) -> OperatorAction {
    OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Near(
        NearArguments::new(
            MemoryRef::parse(anchor).unwrap(),
            dimensions
                .iter()
                .map(|dimension| DimensionRef::parse(*dimension).unwrap())
                .collect(),
            limit.map(|value| PositiveCount::parse(value, "limit").unwrap()),
        )
        .unwrap(),
    )))
}

fn serve_one_json_rpc_response(response: &'static str) -> (String, thread::JoinHandle<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock server");
    let endpoint = format!("http://{}", listener.local_addr().expect("local addr"));
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut bytes = Vec::new();
        let mut buffer = [0_u8; 1024];
        loop {
            let count = stream.read(&mut buffer).expect("read request");
            bytes.extend_from_slice(&buffer[..count]);
            if let Some(header_end) = find_header_end(&bytes) {
                let headers =
                    String::from_utf8(bytes[..header_end].to_vec()).expect("request headers utf8");
                let length = content_length(&headers);
                let body_start = header_end + 4;
                while bytes.len() < body_start + length {
                    let count = stream.read(&mut buffer).expect("read request body");
                    bytes.extend_from_slice(&buffer[..count]);
                }
                let body = String::from_utf8(bytes[body_start..body_start + length].to_vec())
                    .expect("request body utf8");
                let response_bytes = response.as_bytes();
                write!(
                    stream,
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\n\r\n",
                    response_bytes.len()
                )
                .expect("write response headers");
                stream
                    .write_all(response_bytes)
                    .expect("write response body");
                return body;
            }
        }
    });
    (endpoint, handle)
}

fn find_header_end(bytes: &[u8]) -> Option<usize> {
    bytes.windows(4).position(|window| window == b"\r\n\r\n")
}

fn content_length(headers: &str) -> usize {
    headers
        .lines()
        .find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<usize>().expect("content-length usize"))
        })
        .expect("content-length header")
}
