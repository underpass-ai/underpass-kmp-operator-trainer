//! Operator: replay bounded context — application.
//!
//! First-pass scope: the `KmpMcpClient` port + `KmpClientError`. The
//! use cases (`ExecuteReplayUseCase`, replay aggregates, etc.) land in
//! a follow-up PR together with the in-memory + JSON-RPC adapters in
//! `operator-replay-infra`.

pub mod error;
pub mod ports;
