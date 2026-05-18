# Replay Bounded Context

The `replay` bounded context **executes predicted actions against a
real KMP server and records what happened**. It is the first context
that crosses a network boundary; everything else has been in-memory.

Wire format: **MCP JSON-RPC**, not gRPC (see
[ADR 0011](decisions/0011-replay-context-talks-mcp-not-grpc.md)).

## Crates

```
operator-replay-domain       prediction, outcome, execution, report
operator-replay-application  KmpMcpClient port, ExecuteReplayUseCase
operator-replay-infra        InMemoryKmpMcpClient stub (real JSON-RPC client lands later)
```

No `operator-replay-contract` crate today. The wire shapes the JSON-RPC
adapter handles are documented at `api/mcp/README.md`; if a programmatic
schema validator becomes useful, it lands as a follow-up PR.

## Domain map

### Prediction

- `prediction/replay_prediction.rs` — `ReplayPrediction` = trajectory
  id + predicted `OperatorAction`. Independent of evaluation's
  `PredictedAction` per the bounded-context rule.

### Outcome

- `outcome/replay_failure_reason.rs` — `ReplayFailureReason` enum
  (`Transport`, `Protocol`, `InvalidArguments`, `MalformedResponse`).
  Adapter-side `KmpClientError` variants are translated into one of
  these by the use case.
- `outcome/replay_outcome.rs` — `ReplayOutcome` enum:
  - `ToolSucceeded(ToolOutcome)` — the adapter returned a typed
    per-tool outcome.
  - `NoToolCall` — the predicted action was `Stop` or `Escalate`; no
    KMP call was attempted.
  - `ToolCallFailed { tool, reason }` — the adapter returned an
    error; the originally-predicted tool is preserved.

### Execution

- `execution/replay_execution.rs` — `ReplayExecution` pairs a
  prediction with its `ReplayOutcome`.

### Report

- `report/replay_report.rs` — `ReplayReport` aggregates executions,
  exposes `total()`, `successful_tool_calls()`,
  `failed_tool_calls()`, `stop_or_escalate()` and
  `tool_call_success_rate()`. Refuses to be built empty.

### Errors

- `error/replay_domain_error.rs` — `ReplayDomainError` with
  `EmptyReport` plus a transparent `Shared(DomainError)` variant.

## Application map

### Ports

- `ports/kmp_mcp_client.rs` — `KmpMcpClient` trait with one method per
  `KernelTool` (per ADR 0010 §2). Each method takes the typed argument
  value object and returns the per-tool outcome value object from
  `operator-shared-domain::tool_outcomes`.

### Use cases

- `use_cases/execute_replay_use_case.rs` —
  `ExecuteReplayUseCase`. Generic over `C: KmpMcpClient`. Walks the
  predictions; dispatches `ToolCall` actions to the right client
  method; records `Stop` / `Escalate` as `NoToolCall`; translates
  client errors into `ReplayOutcome::ToolCallFailed`; assembles the
  report.

### Errors

- `error/kmp_client_error.rs` — adapter-facing errors
  (`Transport`, `Protocol`, `InvalidArguments`, `MalformedResponse`).
- `error/execute_replay_error.rs` — wraps the rare hard failures
  (today: `EmptyReport`). Per-execution failures land in the
  `ReplayOutcome` of the report, not here.

## Infra map

- `adapters/in_memory_kmp_mcp_client.rs` —
  `InMemoryKmpMcpClient`. Two modes: `ok()` returns canned successful
  outcomes for every tool; `always_failing(FailureMode)` produces
  the matching `KmpClientError`. Used by tests today; the real
  MCP JSON-RPC client (PR C) sits alongside it under a different
  module.

## End-to-end test

`crates/operator-replay-infra/tests/end_to_end.rs` covers four
scenarios:

1. Every-tool happy path — 9 successful tool calls, 100% success
   rate.
2. Stop + Escalate → `NoToolCall` and a 0% rate by convention.
3. Always-failing client — 9 `ToolCallFailed` entries, the originally
   predicted tool preserved per execution.
4. Mixed run — Inspect + Ask succeed, Stop is recorded; totals add up.

## Pending for later passes

- MCP JSON-RPC client (`HttpKmpMcpClient` over `reqwest` or
  `StdioKmpMcpClient` over `tokio::process`). Lands in a follow-up PR
  together with snapshot fixtures from
  `rehydration-kernel/api/examples/kernel/v1beta1/kmp/*.response.json`
  and the first entry in the `api/mcp/` snapshot index.
- Live-server integration test behind a feature flag.
- Per-tool latency tracking on the report.
