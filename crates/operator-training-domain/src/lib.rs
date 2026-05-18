//! Operator training bounded context — domain layer.
//!
//! Owns: training run preparation. Specifically:
//! - `TrainingRunId`, `DatasetProvenance` and `TaskFamilyDistribution`
//!   value objects describing what is going to be trained.
//! - The closed `ReadinessGate` enum, its per-variant evaluator, and
//!   the `ReadinessCheck` / `ReadinessReport` types capturing the
//!   result.
//! - `TrainingManifest` describing the planned run end-to-end.
//! - `TrainingRun` aggregate root, which refuses to be built unless
//!   the manifest's readiness report passes every gate.
//!
//! Out of scope for this crate: I/O (lives in `training-infra`), use
//! cases (live in `training-application`), and the trainer process
//! itself (an external CLI invoked from `training-infra`).
//!
//! See [ADR 0012](../../../docs/architecture/operator/decisions/0012-training-context-design.md).

pub mod errors;
pub mod ids;
pub mod manifest;
pub mod provenance;
pub mod readiness;
pub mod run;
pub mod trainer;
