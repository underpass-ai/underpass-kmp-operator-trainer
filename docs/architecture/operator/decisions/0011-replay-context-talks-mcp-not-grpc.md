# ADR 0011 — Replay context speaks MCP JSON-RPC, not gRPC

Status: accepted (2026-05-18)
Supersedes (in part): [ADR 0010 §1](0010-replay-context-design.md)

## Context

[ADR 0010](0010-replay-context-design.md) locked four decisions for the
upcoming `replay` bounded context. Its **first** decision said the KMP
proto would be vendored at `api/proto/` so `operator-replay-infra`
could compile a gRPC client with `tonic-build`.

Looking at the kernel surface in detail invalidated that decision:

- The kernel exposes both kernel-shaped gRPC services
  (`ContextQueryService`, `ContextCommandService` with `GetContext`,
  `GetContextPath`, `UpdateContext`, …) **and** an operator-shaped one,
  `KernelMemoryService`, whose RPCs map one-to-one onto the operator
  tools (`Wake`, `Ask`, `Goto`, `Near`, `Rewind`, `Forward`, `Trace`,
  `Inspect`, `Ingest`). The latter has existed since kernel commit
  `41e5545` (2026-05-04), predating this ADR — so a tonic-build gRPC
  client *could* in principle be operator-shaped. The choice below is
  therefore not "gRPC can't express the tools"; it is about build cost
  and surface ownership.
- The Operator product is built around tools named `kernel_wake`,
  `kernel_ask`, `kernel_near`, `kernel_inspect`, etc. The kernel already
  serves these tools over MCP JSON-RPC (Model Context Protocol) via the
  kernel crate `rehydration-mcp`, and the MCP payloads carry the richer
  structured shape (`result.structuredContent` / per-tool outcomes) the
  operator consumes. That is the surface the kernel team maintains as
  the operator contract.
- Compiling a `KernelMemoryService` gRPC client would pull `tonic-build`
  + `prost` and a vendored `*.proto` into `operator-replay-infra`,
  and would track a transport the kernel team does not advertise as the
  operator-facing surface. MCP keeps the build light and lets the kernel
  team own the operator-shaped surface.

The previous Operator implementation lived in the kernel workspace and
imported `rehydration-mcp` directly. The new repository forbids that
import (literal `no_kernel_deps` test). The remaining viable option is
to talk the **MCP JSON-RPC wire format** over the network or stdio to
a running kernel MCP server.

## Decision

`operator-replay-infra` speaks **MCP JSON-RPC 2.0** to the kernel's
MCP server. No `.proto` file, no `tonic-build`, no `prost`. The
adapter:

1. Serialises typed `ToolArguments` value objects into the JSON shape
   each MCP tool expects.
2. Sends a JSON-RPC `tools/call` request with `method = "tools/call"`,
   `params.name = "kernel_<tool>"`, `params.arguments = <serialized
   typed args>`.
3. Receives the JSON-RPC response, extracts the
   `result.content[0].text` JSON-encoded structured payload (or the
   newer `result.structuredContent` field, depending on the MCP
   version).
4. Maps the parsed structured payload onto the corresponding per-tool
   outcome value object defined in
   `operator-shared-domain::tool_outcomes`.

Vendoring lives at `api/mcp/`, not `api/proto/`. The directory holds:

- `README.md` — the MCP tool catalog as a frozen reference table plus
  the manual sync policy.
- (Future) `*.schema.json` files — if and when a programmatic
  validator becomes worth maintaining. Not today.

## Points of ADR 0010 that remain valid

- **§2** `KmpMcpClient` port has one method per `KernelTool` — unchanged.
- **§3** Per-tool execution outcomes live in `operator-shared-domain`
  under `src/tool_outcomes/` — unchanged.
- **§4** `no_kernel_deps` architecture test stays literal — unchanged.
  In fact the JSON-RPC path makes this easier: there is **no** Rust
  type from the kernel workspace anywhere in this repository.

Only §1 of ADR 0010 is replaced by this ADR.

## Consequences

Positive:

- No `tonic-build`, no `prost`, no proto-aware build script. The infra
  crate stays simple: `reqwest` (or a stdio runner) plus `serde_json`
  plus the per-tool mappers.
- The MCP surface is exactly what the operator was already designed
  around (`KernelTool` enum, per-tool `ToolArguments`). No abstraction
  mismatch.
- The kernel team owns the gRPC ↔ MCP translation; we consume it.
- Forward-compat: if MCP adds a tool, the operator gets a typed
  compile-fail when `KernelTool::ALL` is extended; the JSON path is
  additive.

Negative:

- MCP wire format is less typed than gRPC. Schema drift in the kernel
  can silently break adapter parsing if the test fixtures we use are
  stale. Mitigation: the per-tool outcome unit tests in
  `operator-replay-infra` will exercise canonical responses copied
  from
  `rehydration-kernel/api/examples/kernel/v1beta1/kmp/*.response.json`
  and assert structural equality.
- JSON-RPC adds about 200 bytes of envelope per call. Negligible for
  replay traffic (10 to 100 actions per trajectory).

## Alternatives considered

- **Stick with gRPC and vendor `*.proto`** (ADR 0010 §1 as written).
  The kernel's `KernelMemoryService` is operator-shaped, so this is
  technically viable. Rejected on cost/ownership grounds: it pulls
  `tonic-build` + `prost` + a vendored proto into the infra crate, gives
  thinner payloads than MCP's structured content, and tracks a transport
  the kernel team does not advertise as the operator-facing surface.
- **Re-implement MCP translation in operator-replay-infra without
  importing rehydration-mcp**. Rejected for the same reason — duplicates
  the kernel's translation logic with no upside.
- **Carve-out in `no_kernel_deps` for a path-dep on `rehydration-mcp`**.
  Rejected because the architectural rule should stay literal; the
  whole point of the independent repo is to not couple to kernel
  crates.
- **Publish `rehydration-mcp` to crates.io and depend on the published
  version**. Premature; revisit only if and when the kernel team
  publishes a stable client.

## When to revisit

- The kernel team publishes a stable MCP client crate on crates.io →
  evaluate switching to a published dep (still no kernel-workspace
  Cargo coupling).
- MCP schema drift breaks our adapter more than twice in a sprint →
  vendor `*.schema.json` files at `api/mcp/` and validate payloads
  programmatically.
- An entirely new tool surface (non-MCP) appears → write a new ADR.
