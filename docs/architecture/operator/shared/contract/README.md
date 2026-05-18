# operator-shared-contract — DTO index

Every type below is a serde DTO. None contains business logic. The crate
depends on `serde` and `serde_json` only.

## DTOs

| Type | File | Wire shape |
| --- | --- | --- |
| `OperatorActionDto` | `operator_action_dto.rs` | tagged union `{"kind": "tool_call"|"stop"|"escalate", ...}` |
| `ToolCallActionDto` | `tool_call_action_dto.rs` | `{"tool": "kernel_near"|..., "arguments": ToolArgumentsDto}` |
| `StopActionDto` | `stop_action_dto.rs` | `{"reason": "...", "answer": optional}` |
| `EscalateActionDto` | `escalate_action_dto.rs` | `{"reason": "...", "target_model": "..."}` |
| `ToolArgumentsDto` | `tool_arguments_dto.rs` | tagged by tool name; matches `KernelTool` strings |
| `CursorDto` | `cursor_dto.rs` | tagged union `{"kind": "ref"|"around"|"temporal"|"trace", ...}` |
| `VisibleStateDto` | `visible_state_dto.rs` | structured snapshot, no `serde_json::Value` field |
| `TrainingTrajectoryDto` | `training_trajectory_dto.rs` | full row shape for JSONL trajectories |

## Stability

Field names and tag strings here are part of Operator's public API. A
breaking change requires a major version bump of the crate and an entry in
this README documenting the migration.

## What does **not** live here

- Validation logic — that is in `operator-shared-domain`.
- Mappers — those translate between DTOs here and domain types over in
  `operator-shared-infra`.
- Convenience constructors that hide invariants — DTOs are pure data.
