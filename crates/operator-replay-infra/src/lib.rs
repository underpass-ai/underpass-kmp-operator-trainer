//! Operator: replay bounded context — adapters.
//!
//! Adapters today:
//! - `InMemoryKmpMcpClient` for unit and integration tests without a
//!   running kernel.
//! - `HttpKmpMcpClient` (this PR) speaking MCP JSON-RPC 2.0 over HTTP
//!   to a kernel MCP server, per ADR 0011.

pub mod adapters;
pub mod jsonrpc;
pub mod mappers;
