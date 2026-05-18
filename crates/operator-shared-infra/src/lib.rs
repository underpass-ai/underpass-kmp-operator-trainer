//! Operator: shared bounded context — infrastructure.
//!
//! Only this crate is allowed to import `serde_json::Value` and to do
//! file system or network I/O within the shared bounded context. See
//! `docs/architecture/operator/shared/infra/README.md` for the per-piece
//! index.

pub mod adapters;
pub mod errors;
pub mod mappers;

#[cfg(test)]
mod round_trip_tests;
