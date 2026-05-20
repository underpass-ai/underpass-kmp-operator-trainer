# Operator KMP Contract P0 Plan — 2026-05-20

## Context

Operator has been moved to its own repository and the new architecture is much
cleaner than the previous attempt:

- independent `operator` repo;
- bounded contexts for shared, synthetic, evaluation, replay and training;
- typed domain value objects;
- DTOs and mappers at the serialization boundary;
- architecture tests enforcing no `serde_json::Value` in domain/application;
- strict policy evaluation through `OperatorActionDto`.

The current blocker is not model training. The blocker is contract alignment.

Before training another 0.5B model, the Operator training contract must match
the real KMP/MCP API that the model is expected to operate. Otherwise the model
will learn a reduced or obsolete API and any benchmark score will be misleading.

## What Was Found

### 1. The v5 handoff dataset is legacy

The handoff points to:

```text
/tmp/kernel-operator-conformance-full-v5/trajectories.jsonl
/tmp/kernel-operator-conformance-full-v5-sft/openai_all.jsonl
/tmp/kernel-operator-conformance-full-v5-sft/openai_eval.jsonl
/tmp/kernel-operator-conformance-full-v5-sft/eval_trajectories.jsonl
```

Those files still use the old action discriminator:

```json
{
  "target_action": {
    "type": "tool_call",
    "tool": "kernel_wake",
    "arguments": {}
  }
}
```

The new Operator contract expects:

```json
{
  "target_action": {
    "kind": "tool_call",
    "tool": "kernel_wake",
    "arguments": {}
  }
}
```

The strict coverage audit fails immediately:

```text
contract-coverage failed: parse line 1: missing field `kind`
```

This is good fail-fast behavior. It prevents us from training on stale data.

### 2. The current Operator contract does not cover all KMP/MCP tools

The real KMP/MCP public surface includes:

```text
kernel_ingest
kernel_wake
kernel_ask
kernel_goto
kernel_near
kernel_rewind
kernel_forward
kernel_trace
kernel_inspect
kernel_write_memory
```

At the start of this P0, `KernelTool` in the new Operator repo only included 9
tools and was missing:

```text
kernel_ingest
```

That is a P0 issue. If we train now, Operator cannot learn the full write API.
It would learn `kernel_write_memory`, but not canonical `kernel_ingest`.

### 3. The Python SFT pipeline still speaks the legacy shape

The Rust contract has moved to `kind`.

The Python scripts still describe and validate actions with `type`:

- `scripts/operator/prepare_operator_sft_dataset.py`
- `scripts/operator/train_operator_sft_lora.py`
- `scripts/operator/predict_operator_sft.py`

The evaluator expects predictions shaped like:

```json
{
  "step_id": "...",
  "action": {
    "kind": "tool_call",
    "tool": "kernel_inspect",
    "arguments": {}
  }
}
```

The predictor still emits/validates the old shape. Training must not start until
the prepare, predict and score stages speak the same contract.

### 4. The runbook is partially stale

`docs/training/runbook.md` already mentions `operator-contract-coverage`, but a
later section still says there is no evaluation CLI yet.

That is documentation drift. It can make the next agent or future us run the
wrong path.

### 5. The kernel repo contains quarantined legacy Operator code

`rehydration-kernel` currently contains a large staged move of old Operator
crates into:

```text
legacy/operator/
```

That code is useful as historical evidence, but it must not become the source of
new Operator architecture. The new repo must stay authoritative.

## Why This Matters

Operator is supposed to learn how to operate KMP through MCP. That only makes
sense if the training corpus has 100% parity with the API/MCP contract.

If the contract is incomplete:

- the model learns missing capabilities;
- coverage numbers are false;
- benchmark results become non-comparable;
- replay against real MCP can fail even if offline evaluation passes;
- we risk repeating the previous architecture failure with patched data and
  hidden compatibility behavior.

This project explicitly wants:

- SOLID;
- DDD with real value objects, not primitive strings everywhere;
- hexagonal boundaries;
- no fallback;
- fail-fast;
- contract coverage as a visible metric.

Training before the contract is closed violates that.

## P0 Plan

### P0.1 — Close the KMP/MCP tool set in domain

Add `kernel_ingest` to the closed Operator tool model:

- `KernelTool::Ingest`;
- `ToolArguments::Ingest`;
- `KmpMcpCapability::Ingest`;
- allowed tools by mode;
- action contract validation;
- serialization DTO;
- DTO to domain mapper;
- domain to DTO mapper;
- replay client port if replay is expected to execute ingest.

Why necessary:

Operator must not reduce the public KMP/MCP API. If `kernel_ingest` is missing,
the model is trained against a mutilated contract.

### P0.2 — Decide the typed shape for `kernel_ingest`

Define a real typed domain shape for ingest arguments. Do not store it as a raw
JSON blob in domain/application.

Minimum contract should reflect the MCP/gRPC API:

```text
about
memory.dimensions
memory.entries
memory.relations
memory.evidence
provenance
idempotency_key
dry_run
```

The shape can be intentionally minimal at first, but it must be typed and must
round-trip through DTOs and mappers.

Why necessary:

`kernel_ingest` is canonical write. If we leave it as arbitrary JSON, the
operator cannot be audited and contract coverage is weak.

### P0.3 — Align action wire format end-to-end

Make the complete training pipeline use the new action discriminator:

```text
kind
```

instead of:

```text
type
```

Affected surfaces:

- trajectory JSONL;
- SFT assistant messages;
- predictor output;
- policy evaluator input;
- baseline LLM output;
- docs/examples.

Why necessary:

The model should learn exactly the output shape accepted by the strict
evaluator and replay runtime. No compatibility shim should hide drift.

### P0.4 — Regenerate datasets from the new contract

Do not patch `/tmp/kernel-operator-conformance-full-v5` row by row.

Generate a new dataset from the new Operator contract and run:

```bash
cargo run --release -p operator-evaluation-cli --bin operator-contract-coverage -- \
  --trajectories /tmp/<new-dataset>/trajectories.jsonl \
  --require-full-coverage \
  --require-zero-invalid
```

Required result:

```text
tool coverage: 10/10
invalid rows: 0
```

Why necessary:

This gives us a hard gate before tokens, GPU time or benchmark time are spent.

### P0.5 — Regenerate SFT only from validated trajectories

Once the trajectory JSONL passes contract coverage, generate the SFT data.

The SFT assistant completion must contain the exact new action:

```json
{
  "action": {
    "kind": "tool_call",
    "tool": "kernel_near",
    "arguments": {}
  }
}
```

Why necessary:

The model learns from the assistant completion. If that completion still uses
legacy `type`, the model learns the wrong contract even if Rust is correct.

### P0.6 — Make predictor validation strict

The predictor must reject:

- `type`;
- unknown tools;
- unsupported prepared actions;
- unbounded calls;
- write calls without the required typed payload;
- actions not allowed by row mode;
- references not present in visible state where the contract requires proof.

Why necessary:

Prediction is the first runtime-like gate after training. It must not “repair”
model output silently.

### P0.7 — Update runbook and handoff docs

Fix stale docs:

- remove the claim that no evaluation CLI exists;
- state that legacy v5 artifacts are incompatible with the current strict
  contract;
- document the required pre-training gates;
- document that `rehydration-kernel/legacy/operator` is historical evidence
  only.

Why necessary:

The next session must not rediscover the same problem or train from the wrong
artifact.

## What We Should Not Do

Do not:

- add a fallback that accepts both `type` and `kind`;
- patch generated rows manually;
- train on `/tmp/kernel-operator-conformance-full-v5-sft` as-is;
- import code from `rehydration-kernel/legacy/operator`;
- hide `kernel_ingest` behind `kernel_write_memory`;
- use raw JSON in domain/application to move faster;
- call a benchmark result valid unless the dataset passed strict coverage first.

## Definition Of Done For This P0

This P0 is done when:

- `KernelTool::ALL` covers the full KMP/MCP tool set;
- `operator-contract-coverage` reports 10/10 tools covered;
- every trajectory parses as `TrainingTrajectoryDto`;
- every target action parses as `OperatorActionDto`;
- generated SFT assistant completions use `kind`;
- predictor outputs use `kind`;
- `operator-policy-eval` can score predictions against the same ground truth;
- runbook and handoff docs describe the current path, not the legacy one.

Only after that should we train the 0.5B model again.

## Updates 2026-05-20-T2

This update closes the two pre-coding blockers raised after reviewing the first
P0 plan. The original diagnosis above remains valid and is intentionally kept
unchanged.

### Blocker 1 Closed: Authoritative `kernel_ingest` Shape

`kernel_ingest` must be derived from the real KMP/MCP API, not from memory or
benchmark convenience.

The authoritative sources are:

| Source | Path | Why it matters |
| --- | --- | --- |
| MCP `tools/list` input schema | `/home/tirso/ai/developents/rehydration-kernel/crates/rehydration-mcp/src/protocol.rs` | Defines the public MCP JSON tool schema exposed to LLM clients. |
| MCP argument mapper to gRPC | `/home/tirso/ai/developents/rehydration-kernel/crates/rehydration-mcp/src/grpc/requests/ingest.rs` | Validates and maps MCP JSON arguments into `KernelMemoryService.Ingest`. |
| gRPC service contract | `/home/tirso/ai/developents/rehydration-kernel/api/proto/underpass/rehydration/kernel/v1beta1/memory.proto` | Defines `KernelMemoryService`, `IngestRequest`, `Memory`, provenance, coordinates, relations and evidence. |
| Proto stability test | `/home/tirso/ai/developents/rehydration-kernel/crates/rehydration-proto/src/kernel_v1beta1_contract_tests.rs` | Guards the service surface and core field names. |

The `IngestRequest` proto fields are:

```text
about
memory
provenance
idempotency_key
dry_run
```

The MCP schema marks these fields as required:

```text
about
memory
idempotency_key
```

`provenance` and `dry_run` are optional at the MCP boundary. In the Operator
domain this means:

```text
about: AboutId
memory: IngestMemory
provenance: Option<IngestProvenance>
idempotency_key: NonEmptyString
dry_run: bool or Option<bool>, depending on whether the domain wants to preserve
         "omitted" separately from "false"
```

Use the MCP mapper behavior as the tie-breaker:

```text
dry_run omitted -> false
```

So the first domain cut should model `dry_run` as a bool with explicit defaulting
in the DTO mapper, not as a hidden fallback in domain.

`Memory` must be typed, not raw JSON:

```text
IngestMemory
  dimensions: Vec<IngestDimension>
  entries: Vec<IngestEntry>
  relations: Vec<IngestRelation>
  evidence: Vec<IngestEvidence>
```

Required by MCP:

```text
memory.dimensions
memory.entries
```

Allowed by current MCP mapper:

```text
memory.dimensions can be empty
memory.entries must be non-empty
memory.relations optional
memory.evidence optional
```

The Operator value objects should follow that behavior exactly:

- dimensions: required list, empty allowed;
- entries: required non-empty list;
- relations: optional/empty list;
- evidence: optional/empty list.

Each sub-shape must be derived from `memory.proto` and `protocol.rs`:

| Operator VO | API source |
| --- | --- |
| `IngestDimension` | `MemoryDimension`: `id`, `kind`, `title`, `metadata` |
| `IngestEntry` | `MemoryEntry`: `id`, `kind`, `text`, `coordinates`, `metadata` |
| `IngestCoordinate` | `TemporalCoordinate`: `dimension`, `scope_id`, optional timestamps, optional `sequence`, optional `rank`, `metadata` |
| `IngestRelation` | `MemoryRelation`: `source_ref`, `target_ref`, `rel`, `semantic_class`, `why`, `evidence`, `confidence`, optional `sequence` |
| `IngestEvidence` | `MemoryEvidence`: `id`, `supports`, `text`, `source`, optional `time`, `metadata` |
| `IngestProvenance` | `MemoryProvenance`: `source_kind`, `source_agent`, `observed_at`, `correlation_id`, `causation_id` |

Intentional omissions must be documented in the VO where they happen. Do not
silently drop fields from the API.

Important boundary rule:

`kernel_write_memory` is a writer-friendly helper that compiles to
`kernel_ingest`. It does not replace `kernel_ingest` in Operator coverage.
Operator must be able to select both tools where the row mode allows it.

### Blocker 2 Closed: Corpus Source For This P0

For this P0, use option **(b)**:

```text
Extend operator-synthetic-domain / operator-synthetic-infra inside the
independent operator repo.
```

Reason:

- this P0 is about KMP/MCP action contract coverage, not benchmark adapter
  export;
- the old kernel conformance exporters are now quarantined under
  `rehydration-kernel/legacy/operator`;
- the legacy v5 artifact still uses `type`, not `kind`;
- moving back into kernel for this P0 would risk reintroducing the old
  architecture failure;
- `operator` already owns the strict `TrainingTrajectoryDto`,
  `OperatorActionDto`, coverage auditor and synthetic context.

The benchmark adapters remain kernel-side later, but they are not the source of
truth for this contract corpus.

Scope impact:

- add `KmpMcpCapability::Ingest`;
- update `KmpMcpCapability::ALL` to 10 capabilities;
- update `KernelTool::ALL` to 10 tools;
- update `AllowedTools` so write/full modes include both `kernel_ingest` and
  `kernel_write_memory`;
- extend `InMemorySyntheticCaseGenerator` only enough to produce valid typed
  conformance rows for every capability;
- keep it fixture/conformance-grade, not benchmark-realistic;
- document that teacher-model-backed generation remains out of scope for this
  P0.

This means P0.1 includes `KmpMcpCapability::Ingest`. It should not be a stub
once P0.2 is started, because the authoritative shape has now been located.

### P0.0 Added: Schema Anchoring

Add this step before P0.1:

1. Copy the source paths above into code comments or module docs where the
   ingest DTO/domain types are introduced.
2. Add tests proving `KernelTool::ALL` matches the actual public MCP tool list:

```text
kernel_ingest
kernel_write_memory
kernel_wake
kernel_ask
kernel_goto
kernel_near
kernel_rewind
kernel_forward
kernel_trace
kernel_inspect
```

3. Add tests proving `operator-contract-coverage` expects 10 tools, not 9.

Why necessary:

This prevents future drift where Operator silently trains against a subset of
KMP/MCP.

### P0.3.5 Added: Small Round-Trip Smoke

After aligning `kind` everywhere and before generating a larger corpus, run a
small round trip with 1-5 trajectories:

```text
operator-synthesize
  -> prepare SFT
  -> predictor stub or operator-llm-baseline --limit 5
  -> operator-policy-eval
```

If the wire contract is broken, it should fail on 5 rows, not after GPU time or
a larger data run.

Required checks:

- SFT assistant completion uses `kind`;
- prediction output uses `kind`;
- `operator-policy-eval` parses predictions without compatibility mode;
- no `type` discriminator is accepted.

### P0.6 Scope Clarified: Python Files

This is not a find-replace from `type` to `kind`.

The following files must be audited explicitly:

```text
scripts/operator/prepare_operator_sft_dataset.py
scripts/operator/train_operator_sft_lora.py
scripts/operator/predict_operator_sft.py
scripts/operator/audit_operator_sft_no_gold.py
scripts/operator/compare_operator_policy_details.py
scripts/operator/deanonymize_operator_predictions.py
```

Rules:

- reject `type`;
- accept `kind`;
- include `kernel_ingest`;
- do not repair model output silently;
- fail fast on unsupported tools;
- fail fast on unbounded calls;
- fail fast when a prepared-write action is used in a profile that does not
  support deterministic prepared-payload execution.

### Baseline Anchor

The next training run must be interpreted against the last failed comparable
baseline:

```text
Qwen/Qwen2.5-0.5B-Instruct + LoRA
dataset: legacy conformance full v4
size: 58 trajectories
result: 24.1% exact-action accuracy
artifact: /tmp/kernel-operator-qwen05-conformance-full-v4-policy-eval.json
```

This number matters because it separates two hypotheses:

- if the next strict run improves sharply, the dataset/contract cleanup mattered;
- if the next strict run stays near 24%, the 0.5B model or task formulation is
  the limiting factor.

Do not compare the next run against publication-grade MemoryArena numbers. This
is a contract-learning baseline, not a benchmark-reader score.

### Revised P0 Order

```text
P0.0  Anchor schema sources and contract expectations
P0.1  Add missing tool/capability: kernel_ingest
P0.2  Type the kernel_ingest subset from the real API schema
P0.3  Align action wire format on kind end-to-end
P0.3.5 Run 1-5 row round-trip smoke
P0.4  Generate new operator-native conformance corpus
P0.5  Generate SFT only from validated trajectories
P0.6  Make Python validation strict, no repair and no compatibility fallback
P0.7  Update runbook, handoff and model-history docs
P0.8  Train only after contract coverage is 10/10 and invalid rows are 0
```

### Updated Definition Of Done

This P0 is done when:

- `KernelTool::ALL` has 10 tools;
- `KmpMcpCapability::ALL` has 10 capabilities;
- `kernel_ingest` is typed from `memory.proto` / MCP `tools/list`;
- `operator-contract-coverage` reports 10/10 tool coverage;
- every trajectory parses as `TrainingTrajectoryDto`;
- every target action parses as `OperatorActionDto`;
- no production script accepts legacy `type`;
- generated SFT assistant completions use `kind`;
- predictor outputs use `kind`;
- a 1-5 row round-trip smoke passes;
- the 24.1% v4 baseline is recorded in `model-history.md`;
- only then is a 0.5B training run allowed.

## Updates 2026-05-20-T3

This update closes the three remaining pre-implementation clarifications and the
replay-scope decision for `kernel_ingest`.

### Aclaración 1 — TemporalCoordinate

The Operator VO should be named:

```text
IngestTemporalCoordinate
```

Reason: `TemporalCoordinate` is the kernel proto name. Prefixing with `Ingest`
keeps the Operator domain explicit that this is the coordinate shape inside
canonical `kernel_ingest`, not every temporal cursor or replay coordinate in
KMP.

The five timestamp fields must be modelled separately:

```rust
pub struct IngestTemporalCoordinate {
    dimension: DimensionRef,
    scope_id: NonEmptyString,
    occurred_at: Option<std::time::SystemTime>,
    observed_at: Option<std::time::SystemTime>,
    ingested_at: Option<std::time::SystemTime>,
    valid_from: Option<std::time::SystemTime>,
    valid_until: Option<std::time::SystemTime>,
    sequence: Option<PositiveCount>,
    rank: Option<PositiveCount>,
    metadata: StringMap,
}
```

`operator-shared-domain` currently has only `thiserror` as a dependency. It does
not depend on `chrono`, so the domain VO should use `std::time::SystemTime`.
The infra mapper can parse RFC3339 DTO strings into `SystemTime` at the
serialization boundary.

Source:

- `memory.proto:51-62` declares `occurred_at`, `observed_at`, `ingested_at`,
  `valid_from`, `valid_until`, `sequence`, `rank`, and `metadata`.
- `rehydration-mcp/src/protocol.rs:532-555` exposes the same coordinate fields
  in the MCP schema.
- `rehydration-mcp/src/grpc/requests/ingest.rs:106-155` maps the five timestamp
  fields independently.

Minimum invariant:

- `dimension` and `scope_id` are required.
- `sequence` and `rank`, when present, must be positive.
- If both `valid_from` and `valid_until` are present, `valid_until` must be
  greater than or equal to `valid_from`.
- Do **not** require `valid_from` when `valid_until` is present in the first
  Operator cut. The kernel mapper does not impose that: it parses
  `valid_from` at `ingest.rs:134-138` and `valid_until` at `ingest.rs:139-143`
  independently. Adding that stricter rule would be an Operator policy decision,
  not a KMP API rule, and should not be hidden inside the first contract port.

### Aclaración 2 — Reglas vacío/no-vacío de IngestMemory

The ingest memory type decisions are grounded in the MCP schema, MCP mapper and
proto surface:

| Campo | Regla del kernel | Línea citada | Tipo en operator domain |
| --- | --- | --- | --- |
| `about` | required non-empty string in MCP mapper; required top-level field in MCP schema; proto field exists on `IngestRequest` | `ingest.rs:20`, `protocol.rs:34-36`, `memory.proto:21-27` | `AboutId` |
| `memory` | required object in MCP mapper; required top-level field in MCP schema; proto field exists on `IngestRequest` | `ingest.rs:21`, `protocol.rs:34-40`, `memory.proto:21-34` | `IngestMemory` |
| `memory.dimensions` | required array, empty allowed by mapper; required by MCP schema; proto repeated field | `ingest.rs:38`, `ingest.rs:63-77`, `protocol.rs:40-55`, `memory.proto:29-34` | `Vec<IngestDimension>` |
| `memory.entries` | required non-empty array; each entry requires `id`, `kind`, `text`, `coordinates`; proto repeated field | `ingest.rs:39`, `common.rs:101-116`, `ingest.rs:90-103`, `protocol.rs:56-75`, `memory.proto:43-49` | `Vec<IngestEntry>` with a constructor invariant requiring non-empty, or a dedicated `NonEmptyVec<IngestEntry>` if introduced |
| `memory.entries[].coordinates` | required non-empty array; each coordinate requires `dimension` and `scope_id` | `ingest.rs:92`, `common.rs:101-116`, `ingest.rs:106-155`, `protocol.rs:67-70`, `protocol.rs:532-555`, `memory.proto:51-62` | `Vec<IngestTemporalCoordinate>` with a constructor invariant requiring non-empty |
| `memory.relations` | optional array; omitted maps to empty; relation item requires `from`, `to`, `rel`, `class`; non-structural relations require confidence and `why` or `evidence` | `ingest.rs:40`, `common.rs:118-130`, `ingest.rs:158-193`, `protocol.rs:76-102`, `memory.proto:64-73` | `Vec<IngestRelation>` |
| `memory.evidence` | optional array; omitted maps to empty; item requires `id` and `text` | `ingest.rs:41`, `common.rs:118-130`, `ingest.rs:196-206`, `protocol.rs:103-121`, `memory.proto:75-82` | `Vec<IngestEvidence>` |
| `idempotency_key` | required non-empty string in MCP mapper; required top-level field in MCP schema; proto field exists on `IngestRequest` | `ingest.rs:25`, `protocol.rs:34`, `memory.proto:21-27` | `NonEmptyString` |
| `provenance` | optional object at MCP boundary; if present, `source_kind`, `source_agent`, and `observed_at` are required by mapper; proto field exists on `IngestRequest` | `ingest.rs:22-24`, `ingest.rs:209-226`, `protocol.rs:124-146`, `memory.proto:84-90` | `Option<IngestProvenance>` |
| `dry_run` | optional bool at MCP boundary; omitted maps to `false`; proto field exists on `IngestRequest` | `ingest.rs:26`, `protocol.rs:34-35`, `memory.proto:21-27` | `bool`; DTO mapper collapses omitted to `false` explicitly |

Notes:

- `common.rs:101-116` is the shared non-empty required array helper.
- `common.rs:118-130` is the shared optional array helper that maps absence to
  an empty slice.
- `ingest.rs:63-77` is the local required-array helper that allows empty
  dimensions specifically.

### Aclaración 3 — Reglas del validador estricto para Ingest

The ingest strict validator must be explicit. It is not enough to add
`ToolArguments::Ingest` and let the current generic specs pass.

Current gap:

`ActionContractSubject` only carries `action`, `mode`, and `visible_state`.
It does not carry the trajectory `about`. Therefore the rule "action about must
match trajectory about" cannot be expressed by the current subject. P0.1/P0.2
must widen the validation subject or add a trajectory-level spec before claiming
strict ingest coverage.

Closed rule set:

| Rule | Source | File where the rule should live |
| --- | --- | --- |
| `kernel_ingest` is only allowed in `OperatorMode::Write` and `OperatorMode::Full`; not in read or writer pre-read profiles. | Operator design; current `AllowedTools` pattern; previous kernel-side docs treat ingest as write canonical. | `crates/operator-shared-domain/src/specifications/tool_within_mode_spec.rs` plus `allowed_tools.rs` update |
| `IngestArguments.about` must equal `TrainingTrajectory.about`. | Operator design to prevent invented abouts; required because kernel API only validates syntactic `about`, not whether the operator was allowed to use it. | New `crates/operator-shared-domain/src/specifications/action_about_matches_trajectory_spec.rs`, requiring validation subject to include `about` |
| `memory.entries` must be non-empty. | MCP mapper uses required non-empty array helper: `ingest.rs:39`, `common.rs:101-116`; MCP schema has `minItems: 1` at `protocol.rs:56-59`. | `crates/operator-shared-domain/src/tool_arguments/ingest_memory.rs` constructor invariant |
| every entry must have at least one coordinate. | MCP mapper: `ingest.rs:92`, `common.rs:101-116`; MCP schema: `protocol.rs:67-70`. | `crates/operator-shared-domain/src/tool_arguments/ingest_entry.rs` constructor invariant |
| every coordinate dimension must reference either a dimension declared in the same ingest action or a dimension already visible in `visible_state.known_dimensions`. | Operator design for anti-hallucination; API source only requires `dimension` as non-empty string (`ingest.rs:109-113`, `protocol.rs:536-539`). | New `crates/operator-shared-domain/src/specifications/ingest_coordinate_dimensions_known_spec.rs` |
| every relation `source_ref` and `target_ref` must reference either an entry id declared in the same ingest action or a ref already visible in `visible_state.known_refs`. | Operator design for anti-hallucination; API source requires `from` and `to` strings at `ingest.rs:184-186`, `protocol.rs:81-85`. | New `crates/operator-shared-domain/src/specifications/ingest_relations_reference_known_refs_spec.rs` |
| non-structural relations require confidence and at least one of `why` or `evidence`. | Kernel mapper rule: `ingest.rs:175-181`; schema documents same intent at `protocol.rs:90-95`. | `crates/operator-shared-domain/src/tool_arguments/ingest_relation.rs` constructor invariant |
| relation `rel` must be a canonical relation value accepted by the kernel relation vocabulary. | Kernel mapper canonicalizes/refuses invalid relation types at `ingest.rs:160-168`. | `crates/operator-shared-domain/src/tool_arguments/ingest_relation_type.rs` value object or equivalent |
| evidence `supports` entries, when present, must point to an entry/ref/relation target visible in this ingest action or `visible_state.known_refs`. | Operator design for proof quality; API only types `supports` as repeated string at `memory.proto:75-82` and parses it at `ingest.rs:196-206`. | New `crates/operator-shared-domain/src/specifications/ingest_evidence_supports_known_refs_spec.rs` |
| `idempotency_key` must be non-empty. | Kernel mapper requires non-empty string at `ingest.rs:25`; proto field at `memory.proto:21-27`. | `crates/operator-shared-domain/src/tool_arguments/ingest_arguments.rs` constructor invariant via `NonEmptyString` |
| `dry_run` must be explicit in the domain value after DTO mapping. | Kernel mapper defaults omitted `dry_run` to false at `ingest.rs:26`; Operator design requires no hidden runtime fallback. | `crates/operator-shared-infra/src/mappers/tool_arguments_mapper.rs` DTO-to-domain mapper test |

Refinement against the proposed rule about `memory.relations[].evidence`:

Do **not** model `MemoryRelation.evidence` as an evidence-id reference. In the
real API it is a free text field:

- proto: `MemoryRelation.evidence` is `string evidence` at `memory.proto:64-73`;
- mapper: it is parsed as optional string and defaulted to empty at
  `ingest.rs:170-172`;
- evidence ids live in `MemoryEvidence.id` at `memory.proto:75-82`.

Therefore the strict validator should check `MemoryEvidence.supports`, not
pretend `MemoryRelation.evidence` is an id pointer.

Composite integration:

`CompositeActionContractValidator::default_strict()` must add the ingest specs
instead of routing `ToolArguments::Ingest` through a generic pass-through path.
The preferred shape is several small specification files, not one mega
validator.

### Decisión — Replay client para ingest

Elección: **A — incluir replay ingest en P0.1**.

Reason:

This P0 is about KMP/MCP contract parity. If evaluation supports 10/10 tools but
replay only executes 9/10, we preserve the same kind of asymmetric gap that the
postmortem rejected. `kernel_ingest` is the canonical write operation, so replay
must either execute it or the training contract is not really end-to-end.

Scope impact in P0.1:

- add `KmpMcpClient::ingest(&IngestArguments) -> Result<IngestOutcome, KmpClientError>`;
- add `ToolOutcome::Ingest`;
- add `IngestOutcome` with at least:
  - `summary`;
  - `about`;
  - `memory_id`;
  - accepted counts if present;
  - `read_after_write_ready`;
  - warnings if present;
- add HTTP MCP request mapper `IngestArguments -> tools/call arguments`
  following `rehydration-mcp/src/protocol.rs:28-123`;
- add response mapper for `structuredContent` following the proto response
  shape at `memory.proto:92-109`;
- add in-memory replay adapter support;
- add mock-server tests following the existing
  `http_kmp_mcp_client.rs` pattern;
- update replay docs to state that P0 replay covers all 10 tools.

The expected scope is small because replay already has the tool dispatch,
request-builder, response-mapper and mock-server pattern. This is not a new
architecture path; it is completing parity for the missing canonical write
tool.

## Updates 2026-05-20-T4

### Implementación — ajuste sobre `about`

During implementation, the proposed rule "`about` ∈
`visible_state.known_refs`" was found to be wrong for the current domain model:

- `AboutId` and `MemoryRef` are different value objects.
- `VisibleState` exposes `known_refs` and `known_dimensions`, but no
  `known_abouts`.
- Treating `about` as a memory ref would weaken the type boundary and re-create
  the primitive/string confusion the refactor is trying to remove.

Implemented rule:

```text
action.about == training_trajectory.about
```

This is enforced by
`ActionAboutMatchesTrajectorySpec`, after widening `ActionContractSubject` to
carry the trajectory `AboutId`.

Current scope:

- enforced for `kernel_ingest`;
- also enforced for `kernel_wake`, because it already carries an explicit
  `AboutId`;
- tools that do not yet carry `AboutId` cannot be checked until their domain
  arguments are expanded.

If the operator later needs "known about" semantics, it should be added as a
first-class `VisibleState.known_abouts: BTreeSet<AboutId>`, not folded into
`known_refs`.

### Implementación — paridad 10/10

Implemented P0.1/P0.2/P0.3 surface:

- `KernelTool::Ingest`;
- `ToolArguments::Ingest`;
- `ToolOutcome::Ingest`;
- `KmpMcpCapability::Ingest`;
- `AllowedTools` now treats `kernel_ingest` as a write/full-mode tool;
- typed ingest DTOs in `operator-shared-contract`;
- typed ingest domain value objects in `operator-shared-domain`;
- `IngestArgumentsMapper` in `operator-shared-infra`;
- strict specs for:
  - action about equality;
  - ingest coordinate dimensions;
  - ingest relation source/target refs;
  - ingest evidence supports refs;
- replay port/client support for ingest;
- HTTP MCP request mapper for ingest;
- HTTP MCP response mapper for ingest;
- synthetic generator coverage now produces 10/10 KMP/MCP capabilities.

### Implementación — mapper boundaries

To avoid a large generic mapper:

- `ToolArgumentsMapper` stays a dispatcher.
- `IngestArgumentsMapper` owns ingest DTO/VO conversion.
- `HttpKmpMcpClient` stays an MCP dispatcher.
- `IngestRequestMapper` owns ingest request JSON construction.

This keeps JSON construction in infra and keeps domain/application free of
`serde_json`.

### Verificación local

Commands run after the implementation:

```text
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

All passed locally.

Architecture checks that passed include:

- no kernel dependencies from operator crates;
- no serde/serde_json in domain/application;
- application ports expose domain types, not primitive/JSON payloads;
- one public type per source file;
- synthetic contract coverage reaches 10/10 KMP/MCP tools.

## Updates 2026-05-20-T5

### Arquitectura — un archivo, un tipo público

The `one_file_one_class` architecture test no longer has a
`KNOWN_EXCEPTIONS` allow-list.

Previously accepted paired public types were split into separate files:

- `CursorKind`;
- `OperatorActionKind`;
- `BudgetField`;
- `BudgetSnapshotDto`;
- JSON-RPC request/response child DTOs;
- `EnvelopeViolation`;
- `FailureMode`.

This makes the rule mechanical and fail-fast:

```text
one source file -> at most one public struct, enum or trait
```

No current source file is exempt.

### Replay — ingest included in every-tool E2E

The replay end-to-end test now exercises all 10 KMP/MCP tools, including
`kernel_ingest`. The previous replay E2E still asserted 9 tool calls, which
was stale after adding ingest to the replay port/client.

Updated verification after this cleanup:

```text
cargo fmt --all --check
cargo test -p operator-architecture-tests
cargo check --workspace
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
git diff --check
```

All passed locally.
