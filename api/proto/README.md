# `api/proto/` — KMP proto snapshots

This directory holds **frozen snapshots** of the `.proto` files that
define the KMP gRPC surface. The Operator workspace consumes them at
build time (via `tonic-build` in `operator-replay-infra`, when that
crate lands).

## Why snapshots and not a dependency

The upstream proto lives at
`rehydration-kernel/crates/rehydration-proto/proto/`. The Operator
architecture forbids any direct Rust dependency on `rehydration-*`
crates (see `operator-architecture-tests/tests/no_kernel_deps.rs` and
ADR 0010). Vendoring the `.proto` files as plain text keeps the rule
literal: the operator workspace compiles its own gRPC client locally
from these snapshots, with no Cargo path-dep or workspace edge to the
kernel repository.

## Sync policy

Manual. To bump a snapshot:

1. Identify the upstream file at
   `https://github.com/underpass-ai/rehydration-kernel/blob/<sha>/crates/rehydration-proto/proto/<file>.proto`.
2. Copy its content into `api/proto/<file>.proto` here.
3. Update the upstream SHA index below.
4. Open a single dedicated PR with subject `chore(api/proto): bump <file>.proto to <short-sha>`.

If the schema change is breaking, the same PR may also adjust
`operator-shared-domain::tool_outcomes::*` and the `KmpMcpClient`
port methods. Do not bundle proto bumps with unrelated work — proto
drift must be visible in `git log`.

## Snapshot index

(Pending — first snapshot lands with the `replay-bootstrap` PR.)

| File | Upstream path | Upstream SHA | Captured at |
| --- | --- | --- | --- |
| _(none yet)_ |  |  |  |

## Build wiring (pending)

When `operator-replay-infra` lands, its `build.rs` will compile every
`.proto` file in this directory:

```rust
fn main() {
    tonic_build::compile_protos("../../api/proto/kmp.proto").unwrap();
}
```

The generated code lives in `OUT_DIR` and is never committed.
