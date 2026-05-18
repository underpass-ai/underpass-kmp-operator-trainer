# ADR 0003 — Typed tool arguments instead of `(tool: &str, arguments: Value)`

Status: accepted (2026-05-18)

## Context

The legacy code expressed a tool call as
`fn tool_call(tool: &str, arguments: serde_json::Value) -> serde_json::Value`.
Helpers like
`fn temporal_call(..., cursor_key: &str, cursor: Value, dimensions: Value,
limit: Value, window: Value)` accumulated. The compiler could not catch
mistakes such as calling `Wake` with a temporal cursor.

## Decision

`ToolCallAction` is **not** a struct with a `tool: KernelTool` field and a
`Value` arguments field. It is an enum whose variant **is** the tool, and
each variant holds a typed arguments value object:

```rust
pub enum ToolCallAction {
    Wake(WakeArguments),
    Ask(AskArguments),
    Near(NearArguments),
    Goto(GotoArguments),
    Rewind(RewindArguments),
    Forward(ForwardArguments),
    Trace(TraceArguments),
    Inspect(InspectArguments),
    WriteMemory(WriteMemoryArguments),
}
```

`ToolCallAction::tool() -> KernelTool` returns the tool identifier for the
current variant. The opposite mapping (`KernelTool` → expected variant) is
intentionally not provided, because the only way to construct a
`ToolCallAction` is by giving its arguments.

Cursors are not strings:

```rust
pub enum Cursor {
    Ref(RefCursor),
    Around(AroundCursor),
    Temporal(TemporalCursor),
    Trace(TraceCursor),
}
```

`TemporalCursor` holds a `TemporalCursorKey` enum (`Created`, `Updated`,
`Accessed`) — no more `cursor_key: &str`.

## Consequences

- Adding a new `KernelTool` variant is a compile-fail across every
  exhaustive match. By design.
- `serde_json::Value` is no longer present in the domain. Mappers in
  `*-infra` translate to and from JSON when crossing the I/O boundary.
- Some fixtures become longer to write. We accept this because correctness
  by construction is worth the verbosity.

## Enforcement

`operator-architecture-tests::no_string_tool_or_cursor` greps for the
patterns `tool: &str` and `cursor_key: &str` in any `.rs` file outside
`*-infra/src/mappers/` and fails if found.
