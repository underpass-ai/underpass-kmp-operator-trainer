//! Operator: shared bounded context — application.
//!
//! See `docs/architecture/operator/shared/application/README.md` for the
//! index of every use case and port defined in this crate. Use cases
//! depend on traits declared in `ports/`; concrete adapters live in
//! `operator-shared-infra`.

pub mod error;
pub mod ports;
pub mod use_cases;
