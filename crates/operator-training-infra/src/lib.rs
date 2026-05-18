//! Operator training bounded context — infrastructure adapters.
//!
//! Implements the `operator-training-application` ports against
//! concrete I/O backends:
//! - `JsonlSftDatasetWriter` — writes one JSONL line per
//!   `TrainingTrajectory` (`{"prompt": …, "completion": …}` per
//!   ADR 0012 §3), SHA-256-hashes the bytes, computes the per-task-
//!   family distribution.
//! - `TomlManifestWriter` — serialises a `TrainingManifest` to a
//!   TOML file (per ADR 0012 §4).
//! - `ProcessTrainerInvoker` — invokes the external trainer
//!   described by a `TrainerTarget` via `std::process::Command`.
//!
//! See [ADR 0012](../../../docs/architecture/operator/decisions/0012-training-context-design.md).

pub mod adapters;
pub mod dto;
