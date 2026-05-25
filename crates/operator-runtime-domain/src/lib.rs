//! Operator: runtime bounded context — domain.
//!
//! Runtime composes an operator policy, contract validation and one KMP/MCP
//! execution step. This crate owns the typed session values only; transport,
//! JSON and model adapters live in infra.

pub mod budget;
pub mod error;
pub mod session;
