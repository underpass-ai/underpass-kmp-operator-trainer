# `api/mcp/` — KMP tool catalog (MCP-shaped reference)

This directory documents the surface the Operator product talks to: the
**Model Context Protocol (MCP)** tools exposed by the KMP server in
`rehydration-kernel`. The transport is JSON-RPC 2.0; there is no
`.proto` file for this layer (see [ADR 0011](../../docs/architecture/operator/decisions/0011-replay-context-talks-mcp-not-grpc.md)).

## Why MCP and not gRPC

The kernel exposes two distinct surfaces:

- A raw gRPC API (`ContextQueryService`, `ContextCommandService`)
  defined at
  `rehydration-kernel/api/proto/underpass/rehydration/kernel/v1beta1/*.proto`.
  Operations are kernel-shaped: `GetContext`, `UpdateContext`,
  `GetNodeDetail`, …
- An MCP JSON-RPC surface defined in
  `rehydration-kernel/crates/rehydration-mcp/src/protocol.rs`.
  Operations are operator-shaped: `kernel_wake`, `kernel_ask`,
  `kernel_near`, `kernel_goto`, `kernel_rewind`, `kernel_forward`,
  `kernel_trace`, `kernel_inspect`, `kernel_write_memory`. The kernel
  also exposes `kernel_ingest`, which is modelled in `KernelTool` but
  has no vendored response fixture in this directory (it is outside the
  frozen-reference table below).

The Operator product is built around the operator-shaped surface (see
the `KernelTool` enum in `operator-shared-domain`). Talking gRPC
directly would require re-implementing the operator-shaped translation
that already lives in `rehydration-mcp` — which would violate the
`no_kernel_deps` architectural test (literally) and the spirit of
"replay belongs in operator" (operationally).

## Tool catalog (frozen reference)

| MCP tool name | Input keys | Output structured_content keys |
| --- | --- | --- |
| `kernel_wake` | `about` (required), `role`, `intent`, `dimensions`, `depth`, `budget` | `summary`, `wake` (objective + open_loops + next_actions + …), `proof`, `warnings` |
| `kernel_ask` | `about` (required), `question` (required), `answer_policy`, `dimensions`, `depth`, `budget` | `summary`, `answer`, `because`, `proof`, `warnings` |
| `kernel_near` | `about` (required, `temporal` shape) | `summary`, `temporal`, `coverage`, `entries`, `proof`, `warnings`, `raw_refs` |
| `kernel_goto` | `about` (required, `temporal` shape) | `summary`, `temporal`, `coverage`, `entries`, `proof`, `warnings`, `raw_refs` |
| `kernel_rewind` | `about` (required, `temporal` shape) | `summary`, `temporal`, `coverage`, `entries`, `proof`, `warnings`, `raw_refs` |
| `kernel_forward` | `about` (required, `temporal` shape) | `summary`, `temporal`, `coverage`, `entries`, `proof`, `warnings`, `raw_refs` |
| `kernel_trace` | `from` (required), `to` (required), `role`, `goal`, `page`, `budget` | `summary`, `trace`, `warnings` |
| `kernel_inspect` | `ref` (required), `include` | `summary`, `object`, `links`, `evidence`, `warnings`, `raw` |
| `kernel_write_memory` | `about` (required), full writer payload | `accepted`, `dry_run`, `summary`, `generated_refs`, `relations`, `relation_quality`, … |

The full schemas live in the Rust code at
`rehydration-kernel/crates/rehydration-mcp/src/protocol.rs`. The
canonical request and response examples live as JSON fixtures at
`rehydration-kernel/api/examples/kernel/v1beta1/kmp/*.{request,response}.json`.

## What this directory contains today

- `README.md` (this file): the table above, the sync policy, and the
  link to the kernel source of truth.
- `examples/kernel/v1beta1/kmp/*.response.json` — frozen response
  fixtures. The per-tool mappers in `operator-replay-infra` are
  tested against these exact bytes; if a kernel response shape drifts
  away from these, the mapper tests break and force a deliberate
  re-snapshot.

What this directory does **not** contain:

- Vendored JSON schema files. Operator does **not** validate JSON-RPC
  payloads against a schema today; the typed value objects in
  `operator-shared-domain::tool_outcomes` are the runtime contract. A
  future PR may add vendored `*.schema.json` files alongside this
  README when a programmatic validator becomes valuable.
- A `.proto` file. There is no protobuf at this layer.

## Sync policy

Manual. Whenever the kernel MCP surface changes (a tool is added,
renamed, or its input/output keys change):

1. Inspect the diff in
   `rehydration-kernel/crates/rehydration-mcp/src/protocol.rs` and
   the affected fixtures under
   `rehydration-kernel/api/examples/kernel/v1beta1/kmp/`.
2. Update the table above with the new key list.
3. If the operator's typed outcomes need to grow a field, do that in
   `operator-shared-domain::tool_outcomes` and submit a dedicated PR
   with subject `chore(api/mcp): bump <tool> to match kernel <short-sha>`.
4. Note the upstream commit SHA in the snapshot index below.

## Snapshot index

| Date | Kernel SHA | Notes |
| --- | --- | --- |
| 2026-05-18 | `fc279eae448b` | Initial snapshot of 9 response fixtures, one per the 9 tools with vendored snapshots, used by the per-tool mappers in `operator-replay-infra::mappers`. `KernelTool` has 10 variants; `kernel_ingest` is modelled there (and has a mapper) but is intentionally not vendored here — it is outside this frozen-reference catalog. |

## Where MCP fits in the dependency graph

```
operator-replay-infra        -> reqwest / stdio JSON-RPC client
operator-replay-application  -> KmpMcpClient port (one method per KernelTool)
operator-shared-domain       -> per-tool outcome value objects
```

`operator-replay-infra` is the only crate that does I/O. It serialises
the typed arguments value object into the JSON shape documented above,
sends the JSON-RPC envelope, parses the response's
`structured_content`, and maps it onto the per-tool outcome value
object. The application use case never sees raw JSON.
