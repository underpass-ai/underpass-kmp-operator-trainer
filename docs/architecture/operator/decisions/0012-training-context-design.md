# ADR 0012 — Training context design (groundwork)

Status: accepted (2026-05-18)

Companion to: pending `docs/architecture/operator/50-training-context.md`

## Context

The `training` bounded context is the last unowned slot in the
operator architecture map (see `01-bounded-contexts.md`). The
postmortem nailed its scope tightly: it owns **training run
preparation** — manifests, dataset provenance, readiness gates, and
metrics — and it does **not** run a model. Running is delegated to an
external trainer wired through a CLI adapter that lives in this
context.

Before any code lands in `training-domain` / `training-application` /
`training-infra`, six design decisions had to be locked in so the
architecture tests, the dependency graph, and the build pipeline all
hold:

1. Which other contexts may `training` depend on?
2. Does `training` run a trainer in-process, or only invoke one?
3. What format does the dataset writer emit?
4. What format does the manifest writer emit?
5. What readiness gates exist in the first cut, and how are they
   modelled?
6. What is the aggregate root, and what invariants does it refuse to
   build?

## Decisions

### 1. `training` depends on `shared` and `evaluation` only

```
training-domain      ──▶  operator-shared-domain
training-application ──▶  operator-shared-domain,
                          operator-evaluation-domain,
                          operator-training-domain
training-infra       ──▶  operator-shared-domain,
                          operator-shared-infra,
                          operator-training-domain,
                          operator-training-application
```

The cross-context dependency is `evaluation-domain`, used only by
readiness gates that read `EvaluationReport` field values. There is no
dependency on `synthetic`, `replay`, or `runtime`. There are no
benchmark adapters in this repository — external benchmark translation
lives in the kernel, not in Operator (see `01-bounded-contexts.md`).

Rejected: depending on `synthetic`. Synthetic produces
`TrainingTrajectory` values, but those live in `shared`. The training
context reads trajectories through the shared aggregate root, never
through synthetic's domain.

Rejected: depending on `replay`. Replay outcomes are about on-device
performance measurement; they belong to a separate (later) loop.
Pulling them into training would entangle dataset assembly with live
behaviour.

### 2. `training` does not run a trainer; it invokes one

`training` writes a `TrainingDataset` file and a `TrainingManifest`
file, then **invokes an external trainer command** through a port in
`training-application`. The trainer process is opaque: this context
does not import vLLM, transformers, axolotl, or any Python crate.

A CLI adapter in `training-infra` builds the command line from the
manifest's `TrainerTarget` and executes it via `std::process::Command`,
returning a typed exit-status outcome. Trainer logs are captured
verbatim; this context does not parse them.

Rejected: an in-process trainer. That would force a Python / FFI
dependency, contradict the postmortem's "Rust-only, no kernel deps"
constraint, and entangle training with the operating-system-level
concerns the external trainer already handles.

### 3. Dataset writer emits JSONL SFT

The first-pass dataset format is **JSON Lines**, one
`TrainingTrajectory` per line, each line shaped as
`{ "prompt": <visible_state_text>, "completion": <action_json> }`.

```jsonl
{"prompt":"role: read\\nknown_refs: [node:1]\\n...","completion":"{\\"kind\\":\\"tool_call\\",\\"tool\\":\\"kernel_inspect\\",\\"arguments\\":{...}}"}
```

The `prompt` is a deterministic text rendering of `VisibleState` (the
exact prompt template the operator policy sees at decision time). The
`completion` is the JSON serialisation of the target `OperatorAction`.
This is the lowest-friction format for SFT trainers (HuggingFace
SFTTrainer, axolotl, llama-factory) and matches the operator-policy
prompt contract.

Other formats — DPO pairs, raw trajectory dumps, parquet, TFRecord —
are deferred to dedicated follow-up PRs that introduce additional
`DatasetWriter` implementations. The port stays format-agnostic; the
JSONL SFT writer is the first concrete adapter.

Rejected: starting with DPO pairs. DPO requires synthesising a
`rejected` variant per trajectory; that infrastructure does not exist
yet and conflating SFT with DPO in PR C inflates scope.

Rejected: a raw trajectory dump that defers all formatting to the
trainer. The point of this context is to be **the source of truth for
what a trainer reads**; pushing formatting downstream re-creates the
"every trainer reinvents the format" problem the postmortem flagged.

### 4. Manifest writer emits TOML

The manifest is a single TOML file alongside the JSONL dataset.
The v1 shape exactly mirrors `TrainingManifest` — no extra metadata
fields. Run timestamps and dataset filesystem paths are surface
concerns of the caller, not of the domain model; if they become
necessary they will be added to `TrainingManifest` in a tracked
follow-up PR with this ADR amended.

```toml
[run]
id = "run:2026-05-18:read-only-baseline"

[dataset]
source           = "synthetic-run:2026-05-18"
content_hash     = "sha256:0123456789abcdef..."
trajectory_count = 1024

[dataset.task_family_distribution]
"read.inspect" = 256
"read.ask"     = 256
"read.wake"    = 256
"read.near"    = 256

[readiness]
overall = "ready"

[[readiness.gate]]
kind   = "minimum_trajectories"
target = "512"
status = "passed"

[trainer_target]
command          = "sft-trainer"
base_model       = "Qwen/Qwen2.5-1.5B-Instruct"
output_directory = "out/run:2026-05-18:read-only-baseline"
```

Note that `target` is rendered as a string so a single TOML table can
hold gates with heterogeneous targets (`PositiveCount` vs.
`PassRatePercent`) without `untagged` enum acrobatics. Failed gates
add a `reason = "..."` field captured verbatim from the domain.

TOML beats JSON for manifests because (a) it is the format the
operator workspace already uses for `Cargo.toml`, (b) it round-trips
with comments preserved when humans edit between runs, (c) it
discourages embedding arbitrary nested structure (a healthy
constraint for a manifest).

Rejected: YAML (indentation pitfalls, anchor footguns, no canonical
Rust crate dominates). Rejected: a second JSON file (no signal benefit
over JSONL).

### 5. Readiness gates are a closed enum, evaluated by the spec pattern

```rust
pub enum ReadinessGate {
    MinimumTrajectories(PositiveCount),
    MinimumTaskFamilyCoverage(PositiveCount),
    MinimumEvaluationPassRate(PassRatePercent),
}
```

Each variant carries the gate's target. `ReadinessGate::evaluate(
provenance: &DatasetProvenance, evaluation: &EvaluationReport)`
returns a `ReadinessCheck` (`Passed { target, actual }` |
`Failed { target, actual, reason }`).

A `ReadinessReport` aggregates `ReadinessCheck` results and exposes
`is_ready() -> bool` (all passed).

Adding a new gate is a deliberate domain change: every exhaustive
`match` on `ReadinessGate` breaks at compile time, by design.
Wildcards (`_ => ...`) on `ReadinessGate` are forbidden, reviewed at
PR time.

Rejected: gates as a trait with dynamic dispatch. Trait objects make
the closed-set guarantee invisible and lose exhaustive-match safety,
which is the main reason the operator codebase uses closed enums for
all classification.

Rejected: gates as predicates against a single combined "training
state" struct. That couples every gate to every field, which would
need to grow with every new gate. The per-variant evaluator is
narrower and self-documenting.

### 6. `TrainingRun` is the aggregate root; refuses to be built unready

```rust
pub struct TrainingRun {
    id: TrainingRunId,
    manifest: TrainingManifest,
}

impl TrainingRun {
    pub fn new(
        id: TrainingRunId,
        manifest: TrainingManifest,
    ) -> TrainingResult<Self> {
        if !manifest.readiness().is_ready() {
            return Err(TrainingDomainError::NotReady { /* ... */ });
        }
        if manifest.run_id() != &id {
            return Err(TrainingDomainError::ManifestRunIdMismatch { /* ... */ });
        }
        Ok(Self { id, manifest })
    }
}
```

Constructor invariants:

- `manifest.readiness().is_ready()` must hold — a `TrainingRun` value
  represents a run that is **allowed to launch**. Failing manifests are
  represented by the manifest itself (which is constructable in any
  state) plus the `ReadinessReport` on it. Only `TrainingRun::new`
  refuses to build.
- The manifest's `run_id` must equal the run's own `id` (defence in
  depth against constructing inconsistent values).

Rejected: making the manifest itself refuse to build when not ready.
Workflows that want to *show* a "would-not-launch" manifest (e.g., for
debugging or for emitting it anyway with `dry_run = true`) need it to
be representable. The unready manifest is a legitimate value; the
unready run is not.

## Consequences

Positive:

- `training-domain` depends only on `shared-domain` and
  `evaluation-domain`. No transitive coupling to synthetic, replay,
  runtime, or any kernel crate.
- The closed `ReadinessGate` enum guarantees that adding a gate forces
  every existing call site to choose an outcome at compile time — the
  same property `KernelTool` and `CursorKind` already give us.
- A JSONL dataset + TOML manifest pair is shippable to any external
  trainer with one path argument; we do not commit the operator to a
  single trainer or framework.

Negative:

- The CLI adapter in `training-infra` runs an external process; that
  is the first place in this codebase where wall-clock behaviour
  matters in tests. Integration tests must use a stub trainer command
  (a no-op shell script written to a `tempfile`) rather than reach for
  a real trainer.
- TOML manifests cannot natively express recursive structures; if a
  future gate needs nested config, we factor it out rather than nest
  it.
- DPO and other paired-preference formats will need their own
  `DatasetWriter` implementations. Acceptable; the port is
  format-agnostic.

## Order of work

The training bootstrap lands across three PRs plus a quality pass,
mirroring the replay cadence:

- **PR A** (this PR): `training-domain` skeleton — value objects, the
  `ReadinessGate` enum and its evaluation surface, the
  `TrainingManifest` value object, the `TrainingRun` aggregate root
  with refusal-to-build invariants, domain errors, and the architecture
  / docs scaffolding.
- **PR B**: `training-application` — use cases (`EvaluateReadiness`,
  `AssembleManifest`, `BuildTrainingRun`) and the input/output ports
  (`TrajectorySource`, `EvaluationReportRepository`, `DatasetWriter`,
  `ManifestWriter`, `TrainerInvoker`). No I/O; in-memory stubs only.
- **PR C**: `training-infra` — filesystem JSONL dataset writer,
  filesystem TOML manifest writer, `std::process::Command` trainer
  invoker. CLI integration test against a stub shell script.
- Quality pass after audit.

## When to revisit

- A second dataset format (DPO, GRPO, raw dump) becomes the primary
  consumer → introduce a `DatasetFormat` enum on the port and route by
  format inside the writer; update this ADR.
- The trainer's exit-status semantics grow richer than success/failure
  (e.g., per-epoch metrics streamed back) → introduce a typed
  `TrainerOutcome` aggregate and split the invoker into a launcher +
  stream-consumer pair.
- A future bounded context (`runtime`, online learning) needs to
  consume the manifest at serving time → publish the manifest schema
  under `api/training/` with a snapshot-index discipline, mirroring
  `api/mcp/`.
