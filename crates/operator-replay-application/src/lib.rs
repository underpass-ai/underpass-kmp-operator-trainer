//! Operator: replay bounded context — application.
//!
//! Hosts the `KmpMcpClient` port, the `ExecuteReplayUseCase` and their
//! error types. Adapters (in-memory stub today, MCP JSON-RPC client
//! later) live in `operator-replay-infra`.

pub mod error;
pub mod ports;
pub mod use_cases;
