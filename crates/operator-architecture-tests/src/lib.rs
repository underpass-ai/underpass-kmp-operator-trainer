//! Helpers shared across the architectural-rule integration tests.
//!
//! Tests live under `tests/`. They use the helpers here to enumerate the
//! workspace and inspect Cargo manifests and Rust source files. No
//! production crate of Operator imports this crate.

pub mod crate_inventory;
pub mod crate_kind;
pub mod source_walker;
pub mod workspace_root;
