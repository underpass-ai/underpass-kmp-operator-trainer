# Runtime Bounded Context

The `runtime` bounded context composes the trained Operator policy, the
strict action contract, a KMP/MCP executor, session budget, observations and
JSONL replay artifacts. The current implementation is an MVP single-step
runtime: one request produces one predicted action, optional one MCP call, and
one `SessionOutcome`.

Wire format to KMP is **MCP JSON-RPC**, not direct gRPC, per
[ADR 0011](decisions/0011-replay-context-talks-mcp-not-grpc.md). The first
live executor uses the existing `rehydration-mcp` stdio server as a process
boundary. Operator crates do not depend on `rehydration-*` crates.

## Crates

```
operator-runtime-domain       session ids, request, budget, observation,
                              execution step, outcome class, outcome
operator-runtime-application  OperatorPolicy, McpExecutor, SessionEventSink
                              ports + RunOperatorSessionSingleStepUseCase
operator-runtime-infra        vLLM OpenAI-compatible policy, MCP executors,
                              JSONL and stderr session event sinks
operator-runtime-cli          operator-session-run composition root
```

There is no `operator-runtime-contract` crate in this cut. Runtime uses
domain types from `operator-shared-domain` and DTO/mappers in infra only.

## Domain map

- `budget/session_budget.rs` — `SessionBudget` tracks
  `calls_remaining` and `tokens_remaining`. `try_consume_call` returns a
  typed error instead of panicking.
- `session/operator_session_id.rs` — typed session id.
- `session/operator_request.rs` — input aggregate for a single runtime
  session: session id, goal, initial visible state, mode, allowed tools,
  initial budget and `AboutId`. Construction rejects write tools outside
  write-capable modes.
- `session/observation.rs` — result of one execution step:
  `ToolResponse`, `ToolError`, or terminal observation.
- `session/execution_step.rs` — predicted action plus observation.
- `session/outcome_class.rs` — closed classification:
  `Completed`, `Escalated`, `BudgetExhausted`, `ContractViolation`,
  `McpExecutionFailure`.
- `session/session_outcome.rs` — final single-step result with predicted
  action, optional observation, elapsed time and final budget.

The domain crate has no serde or serde_json dependency.

## Application map

Ports:

- `OperatorPolicy` — predicts an `OperatorAction` from a
  `CalibrationSubject`.
- `McpExecutor` — executes a non-terminal `OperatorAction` against KMP/MCP
  and returns an `Observation`.
- `SessionEventSink` — observes request receipt, prediction, observation and
  final outcome.

Use case:

- `RunOperatorSessionSingleStepUseCase` — the MVP runtime pipeline:
  1. Build the operator subject from `OperatorRequest`.
  2. Predict one action through `OperatorPolicy`.
  3. Validate the action with the strict shared contract.
  4. Short-circuit terminal `stop` / `escalate` actions locally.
  5. Enforce call budget before MCP execution.
  6. Execute one MCP call through `McpExecutor`.
  7. Build and emit `SessionOutcome`.

There is deliberately no multi-step loop in v0. Multi-step visible-state
updates are a later runtime concern.

## Infra map

- `VllmOpenAiOperatorPolicy` — OpenAI-compatible vLLM client. It sends
  `response_format: {"type":"json_schema"}` with a strict vLLM-oriented
  action schema, `temperature: 0.0`, mTLS client identity when configured,
  and the exact scenario system prompt during replay. It parses the assistant
  content into an action envelope and rejects cross-kind fields before mapping
  to domain.
- `KmpMcpStdioExecutor` — MCP JSON-RPC stdio executor. It spawns
  `rehydration-mcp` per tool call, writes one newline-delimited JSON-RPC
  `tools/call` request, reads one response, validates the envelope with
  `operator-replay-infra` DTOs, maps `isError=true` to `Observation::ToolError`,
  and maps structured successful content through the replay response mappers.
  It defensively rejects write tools before spawning the subprocess.
- `KmpMcpHttpExecutor` — HTTP JSON-RPC executor retained for a future kernel
  HTTP bridge. It is not used by the current live deployment.
- `JsonlSessionEventSink` — writes request, prediction, observation and final
  outcome events to JSONL. The runtime CLI writes one top-level session JSONL
  row per scenario as well.
- `StderrSessionEventSink` — lightweight operational sink for local runs.

The stdio executor translates Operator's temporal cursor anchors to the MCP
shape KMP expects:

- `seq:N` -> `{ "sequence": N }`
- ISO-like `...T...Z` -> `{ "time": ... }`
- otherwise -> `{ "ref": ... }`

This translation is infra-only; the domain still treats temporal anchors as
opaque values.

## CLI

`operator-session-run` is the runtime composition root for replay validation.

Key arguments:

- `--scenario-jsonl`
- `--output-dir`
- `--operator-endpoint`
- `--operator-adapter-id`
- `--operator-client-cert`
- `--operator-client-key`
- `--operator-max-tokens`
- `--kmp-mcp-endpoint`
- `--kmp-mcp-transport stdio|http`
- `--kmp-mcp-stdio-command`
- `--mode`
- `--limit`
- `--filter-tools`

`--mode read_profile` is an aggregate replay filter for the v8.1.2 eval
split: it includes `read` and `writer_pre_read` scenarios while excluding
write tools. It preserves the original mode on each `OperatorRequest`.

## Live replay result

The first live validation processed 222 read-profile scenarios from
`/tmp/operator-sft-v8.1.2/openai_eval.jsonl` against:

- `https://0.5b.llm.underpassai.com/v1` / `operator-v8.1.2`
- `https://rehydration-kernel.underpassai.com` through `rehydration-mcp`

Observed:

- Target/predicted action match: 222/222.
- Terminal escalations: 7/7 succeeded locally.
- MCP attempted: 215.
- MCP completed: 0.
- Failure category after request-shape fixes: 215 KMP `NotFound` responses.

The live path is operational, but the selected eval split uses synthetic refs
that are not loaded in production KMP. The result validates runtime wiring and
schema behavior, not production read success. A production-readiness
`mcp_execution_success_rate` requires either loading the eval fixture graph
into an isolated KMP namespace or replaying scenarios built from live refs.

## Dependency edges

Runtime depends on:

- `operator-shared-domain`
- `operator-shared-contract` / `operator-shared-infra` in infra/CLI only
- `operator-synthetic-domain` / `operator-synthetic-infra` in infra/CLI for
  `CalibrationSubject` DTO mapping and replay input parsing
- `operator-replay-infra` in infra for MCP JSON-RPC envelope DTOs and
  response mappers

Runtime does not depend on `rehydration-*` crates and does not import kernel
protobuf or gRPC clients.
