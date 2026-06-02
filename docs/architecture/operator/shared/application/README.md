# operator-shared-application — Use cases and ports index

This crate orchestrates domain behaviour. It depends on
`operator-shared-domain` and `thiserror`. It does **not** depend on
`serde_json`, `tokio`, `reqwest` or any infra crate.

## Ports (under `ports/`)

| Port | File | Returns |
| --- | --- | --- |
| `TrajectoryReader` | `trajectory_reader.rs` | `Iterator<TrainingTrajectory>` over a source |
| `TrajectoryWriter` | `trajectory_writer.rs` | accepts a `TrainingTrajectory` and persists it |

Both ports return and accept **domain** types. Conversion from/to JSONL is
the implementer's responsibility (in `operator-shared-infra`).

## Use cases (under `use_cases/`)

| Use case | File | Purpose |
| --- | --- | --- |
| `ValidateTrajectoryUseCase` | `validate_trajectory_use_case.rs` | Run an `ActionContractValidator` over a single trajectory and report all violations. |
| `ValidateTrajectoriesFromSourceUseCase` | `validate_trajectories_from_source_use_case.rs` | Drive an `ActionContractValidator` over every trajectory yielded by a `TrajectoryReader` and aggregate the outcomes into a `ValidationReport`. |

Each use case owns its own error and report structs as siblings to its
file (`validate_trajectory_error.rs`, `validation_report.rs`,
`trajectory_validation_failure.rs`).
This keeps the public signature stable when implementation details change.

## Dependency injection

Every use case takes its collaborators by constructor. There is no global
service registry. CLIs are composition roots; this crate has none.

## Errors

`ApplicationError` (in `error/application_error.rs`) wraps domain errors
and use-case-specific errors. It does not wrap infra errors; an infra
adapter that fails is responsible for mapping its error into a domain or
application variant before crossing the port boundary.
