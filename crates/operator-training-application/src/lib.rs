//! Operator training bounded context — application layer.
//!
//! Owns:
//! - Ports the use cases depend on: `TrajectorySource`,
//!   `DatasetWriter`, `ManifestWriter`, `TrainerInvoker`.
//! - Per-port adapter-facing error types plus the use-case-facing
//!   `TrainingApplicationError` umbrella.
//! - Use cases: `EvaluateReadinessUseCase`, `AssembleManifestUseCase`,
//!   `BuildTrainingRunUseCase`, `LaunchTrainingRunUseCase`.
//!
//! Out of scope: I/O. Adapters live in `operator-training-infra`.
//!
//! See [ADR 0012](../../../docs/architecture/operator/decisions/0012-training-context-design.md).

pub mod errors;
pub mod ports;
pub mod use_cases;
