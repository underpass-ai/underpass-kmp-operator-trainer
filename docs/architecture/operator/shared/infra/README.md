# operator-shared-infra — Adapters and mappers index

This crate adapts the outside world to the application ports of
`operator-shared-application`. It is the only place in the shared bounded
context that may import `serde_json`, `serde_json::Value`, `tokio`, file
system or network APIs.

## Mappers (under `mappers/`)

| Mapper | File | Direction |
| --- | --- | --- |
| `OperatorActionMapper` | `operator_action_mapper.rs` | DTO ↔ `OperatorAction` |
| `ToolArgumentsMapper` | `tool_arguments_mapper.rs` | DTO ↔ `ToolArguments` |
| `IngestArgumentsMapper` | `ingest_arguments_mapper.rs` | DTO ↔ `IngestArguments` |
| `CursorMapper` | `cursor_mapper.rs` | DTO ↔ `Cursor` |
| `VisibleStateMapper` | `visible_state_mapper.rs` | DTO ↔ `VisibleState` |
| `TrainingTrajectoryMapper` | `training_trajectory_mapper.rs` | DTO ↔ `TrainingTrajectory` |

Each mapper exposes two associated functions: `to_domain(dto: …) ->
Result<Domain, MappingError>` and `to_dto(domain: &Domain) -> Dto`. Mappers
are not traits and have no state.

## Adapters (under `adapters/`)

| Adapter | File | Implements |
| --- | --- | --- |
| `JsonlTrajectoryReader` | `adapters/jsonl/jsonl_trajectory_reader.rs` | `TrajectoryReader` |
| `JsonlTrajectoryWriter` | `adapters/jsonl/jsonl_trajectory_writer.rs` | `TrajectoryWriter` |

The JSONL adapters use the trajectory DTO defined in
`operator-shared-contract` and the `TrainingTrajectoryMapper` to cross the
boundary.

## Errors (under `errors/`)

`InfraError` is a `thiserror` enum that wraps `std::io::Error`,
`serde_json::Error` and `MappingError`. It is internal to this crate; it
must be translated to `DomainError` or `ApplicationError` before crossing
back into the application layer.

## Composition

This crate is not a composition root. CLIs wire its concrete adapters into
application use cases.
