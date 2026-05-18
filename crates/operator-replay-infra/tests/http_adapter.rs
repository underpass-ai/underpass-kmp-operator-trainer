//! Integration tests for `HttpKmpMcpClient`. Spawns a minimal HTTP
//! server in a thread that returns canned JSON-RPC responses; verifies
//! the adapter's happy path, protocol error path and malformed
//! response path.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::mpsc;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use operator_replay_application::error::kmp_client_error::KmpClientError;
use operator_replay_application::ports::kmp_mcp_client::KmpMcpClient;
use operator_replay_infra::adapters::http_kmp_mcp_client::HttpKmpMcpClient;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::tool_arguments::wake_arguments::WakeArguments;

/// Spawns a single-request HTTP server on `127.0.0.1:0`, returns the
/// URL it is listening on, the `JoinHandle` of the responder thread,
/// and a channel that yields the request body the server received.
fn spawn_mock_server(response_body: String) -> (String, JoinHandle<()>, mpsc::Receiver<String>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
    let port = listener.local_addr().expect("local addr").port();
    let url = format!("http://127.0.0.1:{port}");
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept connection");
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .expect("set read timeout");
        let mut reader = BufReader::new(&mut stream);
        let mut content_length: usize = 0;
        loop {
            let mut line = String::new();
            let bytes = reader.read_line(&mut line).expect("read header");
            if bytes == 0 || line == "\r\n" || line == "\n" {
                break;
            }
            let lower = line.to_ascii_lowercase();
            if let Some(rest) = lower.strip_prefix("content-length:") {
                content_length = rest.trim().parse().unwrap_or(0);
            }
        }
        let mut body_bytes = vec![0u8; content_length];
        reader.read_exact(&mut body_bytes).expect("read body");
        let request_body = String::from_utf8(body_bytes).unwrap_or_default();
        let _ = tx.send(request_body);
        drop(reader);
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            response_body.len(),
            response_body,
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response");
        stream.flush().expect("flush response");
    });
    (url, handle, rx)
}

#[test]
fn happy_path_wake_round_trips_through_real_http() {
    let canned = include_str!("../../../api/mcp/examples/kernel/v1beta1/kmp/wake.response.json");
    let envelope =
        format!(r#"{{"jsonrpc":"2.0","id":1,"result":{{"structuredContent":{canned}}}}}"#);
    let (url, handle, body_rx) = spawn_mock_server(envelope);

    let client = HttpKmpMcpClient::new(url).expect("client builds");
    let outcome = client
        .wake(&WakeArguments::new(AboutId::parse("about:test").unwrap()))
        .expect("wake succeeds");

    assert!(outcome.summary().as_str().starts_with("Continue"));
    assert_eq!(outcome.surfaced_refs().len(), 1);

    // Verify the adapter actually sent a tools/call envelope to the server.
    let request_body = body_rx
        .recv_timeout(Duration::from_secs(5))
        .expect("request body");
    assert!(request_body.contains(r#""method":"tools/call""#));
    assert!(request_body.contains(r#""name":"kernel_wake""#));
    assert!(request_body.contains(r#""about":"about:test""#));
    handle.join().expect("server thread");
}

#[test]
fn jsonrpc_error_envelope_maps_to_protocol_error() {
    let envelope =
        r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"Method not found"}}"#;
    let (url, handle, _rx) = spawn_mock_server(envelope.to_string());

    let client = HttpKmpMcpClient::new(url).expect("client builds");
    let err = client
        .wake(&WakeArguments::new(AboutId::parse("about:test").unwrap()))
        .expect_err("server-side error must surface");

    match err {
        KmpClientError::Protocol { tool, message } => {
            assert_eq!(tool, "kernel_wake");
            assert!(message.contains("Method not found"));
            assert!(message.contains("-32601"));
        }
        other => panic!("expected Protocol error, got {other:?}"),
    }
    handle.join().expect("server thread");
}

#[test]
fn malformed_structured_content_maps_to_malformed_response_error() {
    // structuredContent has no `summary` field; wake mapper rejects.
    let envelope =
        r#"{"jsonrpc":"2.0","id":1,"result":{"structuredContent":{"wake":{"causal_spine":[]}}}}"#;
    let (url, handle, _rx) = spawn_mock_server(envelope.to_string());

    let client = HttpKmpMcpClient::new(url).expect("client builds");
    let err = client
        .wake(&WakeArguments::new(AboutId::parse("about:test").unwrap()))
        .expect_err("malformed response must surface");

    match err {
        KmpClientError::MalformedResponse { tool, message } => {
            assert_eq!(tool, "kernel_wake");
            assert!(message.contains("summary"));
        }
        other => panic!("expected MalformedResponse error, got {other:?}"),
    }
    handle.join().expect("server thread");
}

#[test]
fn legacy_content_array_envelope_is_also_accepted() {
    // Older MCP servers wrap the typed payload in a `content[0].text`
    // JSON-encoded string. The envelope helper accepts both.
    let inner =
        r#"{"summary":"legacy ok","wake":{"causal_spine":[{"evidence_ref":"evidence:legacy"}]}}"#;
    let envelope = format!(
        r#"{{"jsonrpc":"2.0","id":1,"result":{{"content":[{{"type":"text","text":{}}}]}}}}"#,
        serde_json::to_string(inner).unwrap(),
    );
    let (url, handle, _rx) = spawn_mock_server(envelope);

    let client = HttpKmpMcpClient::new(url).expect("client builds");
    let outcome = client
        .wake(&WakeArguments::new(AboutId::parse("about:test").unwrap()))
        .expect("legacy content envelope must round trip");

    assert_eq!(outcome.summary().as_str(), "legacy ok");
    assert_eq!(outcome.surfaced_refs()[0].as_str(), "evidence:legacy");
    handle.join().expect("server thread");
}
