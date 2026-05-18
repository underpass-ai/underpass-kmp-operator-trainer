# operator-shared-domain — Domain index

This crate owns the Operator domain. It depends on `thiserror` and nothing
else.

## Top-level public types

| Concept | Type | File |
| --- | --- | --- |
| Tool identifier | `KernelTool` | `tool/kernel_tool.rs` |
| Allowed tools for a mode | `AllowedTools` | `mode/allowed_tools.rs` |
| Operator mode | `OperatorMode` | `mode/operator_mode.rs` |
| Tool arguments (enum over `KernelTool`) | `ToolArguments` | `tool_arguments/tool_arguments.rs` |
| Tool call action | `ToolCallAction` | `action/tool_call_action.rs` |
| Stop action | `StopAction` | `action/stop_action.rs` |
| Escalate action | `EscalateAction` | `action/escalate_action.rs` |
| Operator action | `OperatorAction` | `action/operator_action.rs` |
| Cursor | `Cursor` | `cursor/cursor.rs` |
| Visible state | `VisibleState` | `visible_state/visible_state.rs` |
| Training trajectory | `TrainingTrajectory` | `trajectory/training_trajectory.rs` |
| Contract violation | `ContractViolation` | `contract/contract_violation.rs` |
| Contract violation code | `ContractViolationCode` | `contract/contract_violation_code.rs` |
| Specification trait | `Specification` | `specifications/specification.rs` |
| Action contract validator trait | `ActionContractValidator` | `contract/action_contract_validator.rs` |
| Composite action contract validator | `CompositeActionContractValidator` | `contract/composite_action_contract_validator.rs` |
| Domain error | `DomainError` | `error/domain_error.rs` |

## Value object families

### Identifiers (under `ids/`)

`StepId`, `AboutId`, `SyntheticCaseId`, `DatasetId`, `TrainingRunId`,
`TrainingTrajectoryId`.

### References (under `value_objects/`)

`MemoryRef`, `DimensionRef`, `TaskFamily`, `ModelId`, `NonEmptyString`,
`PositiveCount`, `ExampleCount`.

### Tool arguments (under `tool_arguments/`)

`WakeArguments`, `AskArguments`, `NearArguments`, `GotoArguments`,
`RewindArguments`, `ForwardArguments`, `TraceArguments`, `InspectArguments`,
`WriteMemoryArguments`.

### Cursors (under `cursor/`)

`RefCursor`, `AroundCursor`, `TemporalCursor`, `TemporalCursorKey`,
`TemporalAnchor`, `TraceCursor`.

### Visible state pieces (under `visible_state/`)

`EvidenceRef`, `BudgetSnapshot`.

### Specifications (under `specifications/`)

`ToolWithinModeSpec`, `ArgumentsReferenceKnownEntitiesSpec`,
`CursorReachableFromVisibleSpec`, `BudgetAllowsActionSpec`, `AndSpec`,
`OrSpec`.

## Invariants enforced by domain constructors

- `KernelTool::Wake` exists in `AllowedTools::for_mode(OperatorMode::Read)`.
- `AllowedTools::for_mode(Read)` does not contain `KernelTool::WriteMemory`.
- `ToolCallAction::Goto(GotoArguments { cursor })` requires `cursor` to be
  one of the known cursor variants — enforced by `Cursor` being a typed
  enum.
- `TrainingTrajectory::new(...)` refuses to build a trajectory when the
  target action's tool is not in the trajectory's allowed_tools, or when
  the target action's referenced cursor/refs are not present in the
  visible state.
- Every value object refuses an empty string.

## What does **not** live here

- JSON parsing or `serde_json::Value` types.
- I/O of any kind.
- Logging.
- Any reference to KMP/MCP transport.
