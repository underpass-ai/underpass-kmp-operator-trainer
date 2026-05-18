# Shared Bounded Context

The `shared` bounded context owns the Operator vocabulary. Every other
context depends on it. It depends on nothing else.

This document indexes every public type, port and adapter of the four crates
that make up `shared`:

- `operator-shared-contract` — DTOs
- `operator-shared-domain` — value objects, entities, aggregates, domain
  services, errors
- `operator-shared-application` — use cases and ports
- `operator-shared-infra` — adapters and mappers

## Crate boundaries

```
operator-shared-contract  ─ depends on: serde, serde_json
operator-shared-domain    ─ depends on: thiserror
operator-shared-application ─ depends on: shared-domain, thiserror
operator-shared-infra     ─ depends on: shared-domain, shared-application,
                                       shared-contract, serde, serde_json,
                                       thiserror
```

## Domain map

### Value objects (one file per type)

Located in `operator-shared-domain/src/value_objects/`:

- `non_empty_string.rs` — generic backing type for every named string ID.
- `positive_count.rs` — `usize` greater than zero.
- `example_count.rs` — non-negative `usize`.
- `memory_ref.rs` — reference to a KMP memory node.
- `dimension_ref.rs` — reference to a memory dimension.
- `task_family.rs` — name of a task family.
- `model_id.rs` — name of a model that may be escalated to.

Located in `operator-shared-domain/src/ids/`:

- `step_id.rs`
- `about_id.rs`
- `synthetic_case_id.rs`
- `dataset_id.rs`
- `training_run_id.rs`
- `training_trajectory_id.rs`

### Tools and modes

Located in `operator-shared-domain/src/tool/`:

- `kernel_tool.rs` — the `KernelTool` enum: `Wake`, `Ask`, `Near`, `Goto`,
  `Rewind`, `Forward`, `Trace`, `Inspect`, `WriteMemory`.

Located in `operator-shared-domain/src/mode/`:

- `operator_mode.rs` — `OperatorMode` enum: `Read`, `Write`, `Full`,
  `WriterPreRead`.
- `allowed_tools.rs` — `AllowedTools` value object derived from
  `OperatorMode`. Hides its internal set.

### Cursors

Located in `operator-shared-domain/src/cursor/`:

- `cursor.rs` — `Cursor` enum: `Ref`, `Around`, `Temporal`, `Trace`.
- `ref_cursor.rs` — `RefCursor` value object.
- `around_cursor.rs` — `AroundCursor` value object with a non-empty
  dimensions vector.
- `temporal_cursor_key.rs` — `TemporalCursorKey` enum: `Created`, `Updated`,
  `Accessed`.
- `temporal_anchor.rs` — `TemporalAnchor` value object (typed wall-clock or
  sequence anchor; today we model it as a non-empty opaque string with
  intent).
- `temporal_cursor.rs` — `TemporalCursor { key, anchor }`.
- `trace_cursor.rs` — `TraceCursor { from, to }`.

### Tool arguments

Located in `operator-shared-domain/src/tool_arguments/`:

- `wake_arguments.rs`
- `ask_arguments.rs`
- `near_arguments.rs`
- `goto_arguments.rs`
- `rewind_arguments.rs`
- `forward_arguments.rs`
- `trace_arguments.rs`
- `inspect_arguments.rs`
- `write_memory_arguments.rs`
- `tool_arguments.rs` — enum that variant-matches the `KernelTool` enum.
  `ToolArguments::tool()` returns the `KernelTool` such that
  `ToolCallAction::new(tool, arguments)` is impossible to misuse: the variant
  IS the tool.

### Actions

Located in `operator-shared-domain/src/action/`:

- `tool_call_action.rs` — wraps a `ToolArguments` value.
- `stop_reason.rs` — typed reason for stopping.
- `stop_action.rs`
- `escalate_reason.rs`
- `escalate_action.rs`
- `operator_action.rs` — enum: `ToolCall(ToolCallAction)`,
  `Stop(StopAction)`, `Escalate(EscalateAction)`.

### Visible state

Located in `operator-shared-domain/src/visible_state/`:

- `visible_state.rs` — `VisibleState` aggregate snapshot value object.
- `visible_state_builder.rs` — internal builder used by mappers in infra to
  assemble a `VisibleState` step by step.
- `evidence_ref.rs` — typed evidence reference (`MemoryRef` plus
  optional dimensions).
- `budget_snapshot.rs` — `BudgetSnapshot { calls_remaining,
  tokens_remaining }`.

### Trajectory

Located in `operator-shared-domain/src/trajectory/`:

- `training_trajectory.rs` — aggregate root. Constructor enforces tool ∈
  allowed_tools, mode ⇔ allowed_tools, visible_state consistency with
  target_action.

### Specifications and contract

Located in `operator-shared-domain/src/specifications/`:

- `specification.rs` — `Specification` trait + `and` / `or` combinators
  returning `AndSpec`, `OrSpec`.
- `tool_within_mode_spec.rs`
- `arguments_reference_known_entities_spec.rs`
- `cursor_reachable_from_visible_spec.rs`
- `budget_allows_action_spec.rs`
- `and_spec.rs`
- `or_spec.rs`

Located in `operator-shared-domain/src/contract/`:

- `contract_violation.rs` — `ContractViolation { code, field, expected,
  actual }`.
- `action_contract_validator.rs` — trait + default composite that wires the
  specifications above.

### Errors

Located in `operator-shared-domain/src/error/`:

- `domain_error.rs` — public `DomainError` enum (thiserror).
- `domain_result.rs` — `type DomainResult<T> = Result<T, DomainError>;`.

## Contract map

Located in `operator-shared-contract/src/`:

- `operator_action_dto.rs` — enum DTO matching `OperatorAction`.
- `tool_call_action_dto.rs`
- `stop_action_dto.rs`
- `escalate_action_dto.rs`
- `visible_state_dto.rs`
- `cursor_dto.rs`
- `tool_arguments_dto.rs`
- `training_trajectory_dto.rs`

Every DTO has a stable serde representation. The schemas are documented in
[shared/contract/README.md](shared/contract/README.md).

## Application map

Located in `operator-shared-application/src/ports/`:

- `trajectory_reader.rs` — `TrajectoryReader` trait.
- `trajectory_writer.rs` — `TrajectoryWriter` trait.

Located in `operator-shared-application/src/use_cases/`:

- `validate_trajectory.rs` — `ValidateTrajectoryUseCase`. Input is a domain
  `TrainingTrajectory`, output is the trajectory + applied validator id.

Located in `operator-shared-application/src/error/`:

- `application_error.rs`

## Infra map

Located in `operator-shared-infra/src/mappers/`:

- `operator_action_mapper.rs`
- `visible_state_mapper.rs`
- `training_trajectory_mapper.rs`
- `cursor_mapper.rs`
- `tool_arguments_mapper.rs`

Located in `operator-shared-infra/src/adapters/jsonl/`:

- `jsonl_trajectory_reader.rs`
- `jsonl_trajectory_writer.rs`

Located in `operator-shared-infra/src/errors/`:

- `infra_error.rs`

## Architectural tests

`operator-architecture-tests` exercises every rule in
[00-principles.md](00-principles.md) and asserts:

- no operator crate manifest contains `rehydration-`
- no domain or application manifest contains `serde_json`
- no domain or application source file contains the string `json!`
- no domain or application source file contains parameter patterns
  `tool: &str` or `cursor_key: &str`
- every operator crate has at most one type defined per file (counting
  `pub struct`, `pub enum`, `pub trait` declarations).
