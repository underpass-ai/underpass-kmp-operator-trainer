# ADR 0010 — Replay context design (groundwork)

Status: accepted (2026-05-18); **§1 superseded by
[ADR 0011](0011-replay-context-talks-mcp-not-grpc.md)**.

§2, §3 and §4 remain in force. §1 originally specified that the KMP
proto would be vendored at `api/proto/` and `replay-infra` would
compile a gRPC client. After inspecting the actual kernel surface we
found that operator-shaped tools (`kernel_wake`, `kernel_ask`, etc.)
live on the MCP JSON-RPC layer, not on the raw gRPC layer. ADR 0011
replaces §1 with "vendor the MCP tool catalog at `api/mcp/` and speak
JSON-RPC".

Companion to: pending `docs/architecture/operator/40-replay-context.md`

## Context

The `replay` bounded context is the first that talks to a real external
system: it executes predicted `OperatorAction` values against a live KMP
gRPC server and records the observed outcome. Before any code lands in
`replay-domain` / `replay-application` / `replay-infra`, four design
decisions had to be locked in so the architecture tests, the dependency
graph and the build pipeline all hold:

1. Where does the KMP proto live in this repository?
2. What shape does the `KmpMcpClient` port take?
3. Where do `KernelTool` execution outcomes live?
4. Does `no_kernel_deps` (architecture test) get a carve-out?

## Decisions

### 1. KMP proto lives at `api/proto/` as a versioned snapshot

The directory `api/proto/` at the repository root holds the `.proto`
files copied from `rehydration-kernel/crates/rehydration-proto/proto/`.
Each file is a frozen snapshot. To bump:

1. Copy the upstream `.proto` over the local file.
2. Note the upstream commit SHA in `api/proto/README.md`.
3. Commit the change as a single dedicated PR (so the diff is easy to
   review).

There is no automated sync. Manual coordination is acceptable for a
unit-of-one consumer.

Rejected: git submodule (build complexity > benefit), path-dep on
`rehydration-proto` (breaks `no_kernel_deps`), published crate (kernel
team has not published `rehydration-proto`).

### 2. `KmpMcpClient` port has one method per `KernelTool`

```rust
pub trait KmpMcpClient: std::fmt::Debug + Send + Sync {
    fn ingest(&self, args: &IngestArguments) -> Result<IngestOutcome, KmpClientError>;
    fn wake(&self, args: &WakeArguments) -> Result<WakeOutcome, KmpClientError>;
    fn ask(&self, args: &AskArguments) -> Result<AskOutcome, KmpClientError>;
    fn near(&self, args: &NearArguments) -> Result<NearOutcome, KmpClientError>;
    fn goto(&self, args: &GotoArguments) -> Result<GotoOutcome, KmpClientError>;
    fn rewind(&self, args: &RewindArguments) -> Result<RewindOutcome, KmpClientError>;
    fn forward(&self, args: &ForwardArguments) -> Result<ForwardOutcome, KmpClientError>;
    fn trace(&self, args: &TraceArguments) -> Result<TraceOutcome, KmpClientError>;
    fn inspect(&self, args: &InspectArguments) -> Result<InspectOutcome, KmpClientError>;
    fn write_memory(&self, args: &WriteMemoryArguments) -> Result<WriteMemoryOutcome, KmpClientError>;
}
```

Rejected: polymorphic `execute(action: &OperatorAction) -> Result<ExecutionOutcome, _>`.
That shape forces callers to `match` on the outcome union to extract
typed results (primitive obsession one level up), and silently dispatches
new tools through `match` arms rather than failing at compile time when
an adapter does not implement a variant.

Rejected: per-tool sub-traits (`Wakeable`, `Askable`, …). 10 traits
multiplies type-parameter bounds everywhere downstream. Every real KMP
server we will ever talk to implements all tools; partial implementations
are not a real shape.

### 3. Per-tool execution outcomes live in `operator-shared-domain`

Each `KernelTool` variant gets a typed outcome value object placed
alongside its `ToolArguments` sibling under
`crates/operator-shared-domain/src/tool_outcomes/`:

```
tool_outcomes/
├── mod.rs
├── ingest_outcome.rs
├── wake_outcome.rs
├── ask_outcome.rs
├── near_outcome.rs
├── goto_outcome.rs
├── rewind_outcome.rs
├── forward_outcome.rs
├── trace_outcome.rs
├── inspect_outcome.rs
├── write_memory_outcome.rs
└── tool_outcome.rs  (typed union over all per-tool outcomes)
```

Why shared and not replay-domain? The KMP outcome is a vocabulary item:
synthetic uses it when describing fixture observations, replay returns
it from the adapter, and a future training context will use it as a row
column. Co-locating outcomes with their arguments mirrors the typed
arrangement that already works for `ToolArguments`.

The concrete shape of each outcome is **out of scope for this ADR**.
The replay-bootstrap PR defines the fields; the present ADR fixes only
their location and the per-tool granularity.

### 4. `no_kernel_deps` architecture test stays literal

The test (`crates/operator-architecture-tests/tests/no_kernel_deps.rs`)
asserts that no Operator crate Cargo.toml contains the substring
`rehydration-`. It continues to hold after replay lands:

- The proto snapshot is a local file, not a path-dep on
  `rehydration-proto`.
- `tonic-build` in `replay-infra/build.rs` reads the local snapshot.
- The generated gRPC client code lives inside `replay-infra` (via
  `OUT_DIR`); no external Rust crate from the kernel workspace is
  imported.

Rejected: a carve-out for a hypothetical `rehydration-proto-mirror`
crate. If we ever need the kernel proto as a published artifact, we
publish it as a separately-owned crate (for example
`underpass-kmp-proto` on crates.io) and amend the architecture test at
that point. That work is out of scope until proto changes become
painful enough to motivate it.

## Consequences

Positive:

- `replay-domain` depends only on `operator-shared-domain`.
- `replay-application` depends on `replay-domain` and on a
  `KmpMcpClient` trait. No transitive kernel coupling.
- `replay-infra` is the only crate that uses `tonic-build` + `prost`.
- Proto drift becomes a visible commit, not a silent transitive bump.
- The architectural test surface does not change.

Negative:

- ~10 new value objects in `shared-domain` (one outcome per tool) plus
  associated error types. Acceptable; mirrors the existing
  `ToolArguments` arrangement.
- Manual proto sync is a chore. Acceptable for a unit-of-one consumer.
- Per-tool methods on `KmpMcpClient` are verbose. Acceptable: explicit
  beats clever and every method's signature documents its arguments
  and return type at a glance.

## Order of work for the replay-bootstrap PR

This ADR landing does not block until the next PR. When `replay`
begins, the order is:

1. Add the proto snapshot under `api/proto/` (single dedicated commit
   with the upstream SHA in `api/proto/README.md`).
2. Define per-tool outcome value objects in
   `operator-shared-domain/src/tool_outcomes/`.
3. Define the `KmpMcpClient` trait + `KmpClientError` in
   `operator-replay-application/src/ports/`.
4. Implement `replay-domain` types (ReplayExecution, ReplayReport,
   etc.).
5. Implement the use case in `replay-application`.
6. Implement `TonicKmpMcpClient` adapter in `replay-infra`.
7. End-to-end test that does NOT require a live KMP server: a stub
   adapter that returns canned outcomes.

A live-server integration test is its own follow-up PR with a feature
flag, since the CI environment does not have a kernel deployed.

## When to revisit

- The proto file diverges between this repo and the kernel for more
  than two sprint cycles → switch to a published proto crate and update
  this ADR + the `no_kernel_deps` test accordingly.
- More than one external system needs the same "snapshot + tonic-build"
  pattern → factor the build script boilerplate into a shared crate.
- The per-tool outcome value objects grow shared logic → introduce a
  domain service in `shared-domain`, do not bypass the typed boundary.
