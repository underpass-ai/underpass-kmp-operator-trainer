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
operator-replay-infra        InMemoryKmpMcpClient stub + HttpKmpMcpClient
                             (real MCP JSON-RPC over HTTP) + per-tool
                             response mappers + JSON-RPC envelope DTOs
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
  the matching `KmpClientError`. Used by tests in this crate and as
  a stub by downstream contexts.
- `adapters/http_kmp_mcp_client.rs` — `HttpKmpMcpClient`. Real MCP
  JSON-RPC 2.0 client over HTTP (`reqwest::blocking`). One
  `KmpMcpClient` method per kernel tool. Each method builds a
  `tools/call` envelope, POSTs to the configured endpoint, deserialises
  the response into `ToolsCallResponse`, extracts the structured
  payload, and hands it off to the per-tool response mapper. Errors
  flow as: HTTP non-2xx and JSON-RPC `error` → `Protocol`; transport
  failure → `Transport`; structured-content parse failure or mapping
  error → `MalformedResponse`. Constructed with `new(endpoint)` (30 s
  default timeout) or `with_client(endpoint, client)` to inject a
  caller-built `reqwest::blocking::Client` (custom TLS, proxies, etc.).
  Request id generation is internal, monotonic, and lock-free
  (`AtomicU64`).
- `jsonrpc/tools_call_request.rs`, `jsonrpc/tools_call_response.rs` —
  serde DTOs for the JSON-RPC 2.0 `tools/call` envelope. The response
  helper `structured_content()` accepts both the modern
  `result.structuredContent` field and the legacy
  `result.content[0].text` JSON-encoded payload that older MCP servers
  return, so the adapter is wire-compatible with both shapes. These
  two files are listed in the `one_file_one_class` architecture test's
  `KNOWN_EXCEPTIONS` allow-list as intrinsically paired envelope DTOs.
- `mappers/*_response_mapper.rs` — one mapper per tool. Each takes the
  structured `serde_json::Value`, validates required fields, and
  returns the typed `*Outcome` value object from
  `operator-shared-domain::tool_outcomes`. Mapping failures surface
  as `MappingError` (`MissingField`, `WrongType`, `InvalidValue`),
  which the adapter translates to `KmpClientError::MalformedResponse`.
  Every mapper has a fixture-driven unit test that includes the
  canonical kernel response from `api/mcp/examples/kernel/v1beta1/kmp/`
  at compile time via `include_str!`, so doc drift between operator
  and kernel is caught as a test failure.

## End-to-end test

`crates/operator-replay-infra/tests/end_to_end.rs` covers four
scenarios against `InMemoryKmpMcpClient`:

1. Every-tool happy path — 9 successful tool calls, 100% success
   rate.
2. Stop + Escalate → `NoToolCall` and a 0% rate by convention.
3. Always-failing client — 9 `ToolCallFailed` entries, the originally
   predicted tool preserved per execution.
4. Mixed run — Inspect + Ask succeed, Stop is recorded; totals add up.

## HTTP adapter integration test

`crates/operator-replay-infra/tests/http_adapter.rs` exercises
`HttpKmpMcpClient` against a single-request mock HTTP server built
on `std::net::TcpListener` (no third-party HTTP server dependency in
tests). The mock binds an ephemeral port, captures the request body
that the adapter sent, and replies with a caller-supplied JSON-RPC
envelope. The four scenarios cover:

1. Happy path — canned `wake.response.json` round-trips end to end;
   the captured request body confirms a real `tools/call` envelope
   was sent with the right `name` and `about` arguments.
2. JSON-RPC error envelope — `error.code = -32601` maps to
   `KmpClientError::Protocol` and the message includes both the
   server-side text and the numeric code.
3. Malformed `structuredContent` — a payload missing `summary` maps
   to `KmpClientError::MalformedResponse` with the missing field
   surfaced in the message.
4. Legacy `content[0].text` envelope — older MCP servers wrap the
   typed payload in a JSON-encoded text content block; the envelope
   helper accepts this shape and the adapter still produces a typed
   outcome.

## Pending for later passes

- Live-server integration test against an actual kernel MCP server,
  behind a feature flag.
- Domain modelling of the temporal-anchor shape used by `kernel_near`
  / `goto` / `rewind` / `forward` (`{ time | sequence | ref }`), and
  of the `about` argument on `kernel_ask`. Documented as known
  impedance gaps inline in `http_kmp_mcp_client.rs`.
- Per-tool latency tracking on the report.
