# Operator v7.3 recovery plan — 2026-05-22

## Context

The first full paid v7.3 run (`realistic-v7-full-20260522T173129Z`) processed all 1650 scenarios but failed the corpus gate:

```
total_scenarios: 1650
accepted_count:  1528
dropped_count:   122
drop_rate:       7.39%
max_drop_rate:   5.00%
gate:            FAIL
```

Drops categorize cleanly into three root causes:

| Category | Drops | % of total | Root cause |
|---|---|---|---|
| Ref convention inconsistency | 42 | 34% | scenarios use `node:X` refs; prompt examples use `about:id:node:X` form. Teacher modifies refs (strip `node:` or add `about:`) → `UnknownMemoryRef` violations |
| Wire format violations | 43 | 35% | Teacher emits `{"kind":"kernel_rewind", ...}` directly instead of `{"kind":"tool_call","tool":"kernel_rewind",...}` despite v5 prompt warning |
| Adversarial mistargets | 37 | 30% | Adversarial templates where goal text contradicts expected target. Egregious case: `kernel_ask:no-relevant-memory` (25/25 dropped) — goal says "honesty may require stopping", target=kernel_ask |

Additionally: 3 templates drop at >85% rate (`kernel_ask:no-relevant-memory` 100%, `kernel_rewind:missing-anchor` 96%, `kernel_near:candidate-refs` 88%). Per-template bias makes the 1528 accepted rows unsuitable for training as-is — entire policy categories nearly absent.

The gate did its job. Recovery requires three focused fixes plus observability infrastructure missing from the original run (2+ hours of opaque execution).

## Plan summary — 3 focused PRs

| PR | Fix | Work | LLM cost |
|---|---|---|---|
| #36 | `CorpusEventSink` observability port | 4-6h | $0 |
| #37 | scenarios-v3 (refs + adversarials) | 3-4h | $0 |
| #38 | Structured output enforcement | 1-2h + verify | $1-2 |

All three are architecturally independent enough to review separately. **All three must land before the next full paid run.** Development order should optimize risk reduction: fix deterministic data first, prove structured output against the API second, then add observability before the next paid run.

## Anti-patterns (apply to all 3 PRs)

- ❌ Do not raise `max_drop_rate` above 5%. The gate is correct.
- ❌ Do not iterate the calibration prompt to v6. v5 is the final calibrated prompt.
- ❌ Do not add retry-on-failure logic to the teacher policy. Cost grows; encodes "model corrects after critique" which is not the policy we want trained into the 0.5B.
- ❌ Do not silently fall back to unstructured mode if structured output fails. Loud error.
- ❌ Do not combine the three PRs into one. Separate architectural concerns deserve separate review surfaces.
- ❌ Do not execute the next full re-run until all three PRs are merged.
- ❌ Do not skip the `--limit 30` smoke after the three PRs merge. The next full run only starts after a green smoke with scenarios-v3 + structured output + observability.

---

# PR #36 — `CorpusEventSink` observability port

## Diagnosis

`BuildRealisticCorpusUseCase::execute()` accumulates `accepted` and `dropped` in in-memory `Vec<>`s and writes atomically at the end of the loop. Zero runtime observability. For 2-4 hour paid runs this is operationally untenable.

## Decision 1 — port lives in synthetic-application

New file `crates/operator-synthetic-application/src/ports/corpus_event_sink.rs`:

```rust
use crate::ports::scenario::Scenario;
use crate::error::corpus_event_sink_error::CorpusEventSinkError;
use crate::use_cases::drop_entry::DropEntry;
use crate::use_cases::realistic_corpus_report::RealisticCorpusReport;
use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;

pub trait CorpusEventSink: std::fmt::Debug + Send + Sync {
    fn on_run_started(&self, total_scenarios: usize) -> Result<(), CorpusEventSinkError>;
    fn on_row_accepted(
        &self,
        index: usize,
        total: usize,
        scenario: &Scenario,
        trajectory: &TrainingTrajectory,
    ) -> Result<(), CorpusEventSinkError>;
    fn on_row_dropped(
        &self,
        index: usize,
        total: usize,
        scenario: &Scenario,
        entry: &DropEntry,
    ) -> Result<(), CorpusEventSinkError>;
    fn on_run_finished(&self, report: &RealisticCorpusReport) -> Result<(), CorpusEventSinkError>;
}
```

New error file `crates/operator-synthetic-application/src/error/corpus_event_sink_error.rs`.

The sink is deliberately fallible. Observability is not best-effort for paid runs: if the streaming writer cannot write, flush or sync, the run must fail explicitly. No `panic`, no silent loss of audit rows.

## Decision 2 — use case takes sink by constructor (DI)

Extend `BuildRealisticCorpusUseCase` to `<S, T, V, E>` generic with `E: CorpusEventSink`. Call sink at three points: `on_run_started` before loop, `on_row_accepted`/`on_row_dropped` per iteration, `on_run_finished` after loop. The existing return value (`RealisticCorpusReport`) stays unchanged.

If any sink method returns `Err`, `execute()` returns `BuildRealisticCorpusError::EventSink(...)`. That is an infrastructure failure, distinct from a completed run whose `gate_passed` is false.

## Decision 3 — three adapters in synthetic-infra

New files under `crates/operator-synthetic-infra/src/adapters/`:

### `stderr_progress_sink.rs`

Rolling progress to stderr. Configurable cadence: `every_n_rows: usize, every_duration: Duration`. Default: 25 rows or 30 seconds, whichever comes first. Emit JSON-formatted lines on stderr:

```jsonc
{"event":"realistic_corpus.progress","index":250,"total":1650,"outcome":"accepted","target":"kernel_inspect","progress_pct":15}
```

Always emit drops (not throttled — drops are signal, not noise):

```jsonc
{"event":"realistic_corpus.drop","index":42,"total":1650,"scenario_id":"...","target":"kernel_near","reason_kind":"contract_violation"}
```

Run lifecycle events (`run_started`, `run_finished`) always emit.

### `jsonl_streaming_sink.rs`

Per-row writes to disk. Two `BufWriter<File>` instances guarded by `Mutex`. On `on_row_accepted`: serialize trajectory via `TrainingTrajectoryMapper::to_dto`, write line, `flush()` immediately. Same for drops.

**Critical:** files are named `trajectories.partial.jsonl` and `dropped.partial.jsonl` during execution. The CLI bin renames to final names after the run completes, even if `gate_passed=false`. A failed gate is a completed, auditable result. `.partial.jsonl` remains only for crash/error paths where the run did not complete.

On `on_run_finished`: `sync_all()` on both files before returning.

### `composite_corpus_event_sink.rs`

Multicaster. Holds `Vec<Box<dyn CorpusEventSink>>`, delegates each event to all sinks. Allows wiring stderr + jsonl streaming simultaneously.

### `null_corpus_event_sink.rs`

No-op sink for tests and production runs that don't want observability overhead. Implementation: all trait methods empty bodies.

## Decision 4 — CLI wiring

`operator_realistic_corpus.rs::run()` changes:

1. `fs::create_dir_all(&cli.output)` moves BEFORE `use_case.execute()` (currently after — that's why the output dir is empty during the run)
2. Compose: `CompositeCorpusEventSink::new(vec![Box::new(StderrProgressSink::default()), Box::new(JsonlStreamingSink::new(&cli.output)?)])`
3. Pass sink to use case constructor
4. After `execute()` returns a completed `RealisticCorpusReport`, rename `.partial.jsonl` → final names even when `report.gate_passed() == false`
5. Write `report.json` last

On `execute()` error path caused by infrastructure or event-sink failure: `.partial.jsonl` files remain, no rename happens, and `report.json` is not written. Discoverable via `ls`.

## Decision 5 — existing tests adapt to `NullCorpusEventSink`

Every existing test that constructs `BuildRealisticCorpusUseCase` (the use case unit tests, the CLI smoke, the integration test from PR #35) passes `NullCorpusEventSink` as the fourth generic. No behavioral change to those tests.

## Acceptance criteria

- `cargo build --workspace` green
- `cargo clippy --workspace --all-targets -- -D warnings` green
- `cargo test --workspace` green including existing tests adapted to `NullCorpusEventSink`
- New unit tests:
  - `RecordingSink` stub records the 4 event types; verify they're invoked correctly
  - `StderrProgressSink` emits cadence respects N rows AND T seconds
  - `JsonlStreamingSink` writes per-row, flushes per-row, `sync_all` on finish, and returns `CorpusEventSinkError` on I/O failure
  - `CompositeCorpusEventSink` delegates to all wrapped sinks
- CLI smoke: run with stub teacher + `JsonlStreamingSink` → `trajectories.partial.jsonl` appears with content during execution; renames to `trajectories.jsonl` after completion
- CLI gate-failed smoke: run with stub teacher that exceeds `max_drop_rate`; final `trajectories.jsonl`, `dropped.jsonl` and `report.json` are still written, and `gate_passed=false`
- One-file-one-class architecture test stays green (4 new public types in 4 separate files)

## Scope NOT in this PR

- Parallelization of the use case loop (separable, future)
- Changes to `process_one` logic
- Python pipeline changes
- Scenarios or prompt changes

## Estimated effort

4 new infra files + 1 new application port + use case generic param + CLI bin wiring + ~150 LOC tests. ~400-600 LOC total. 4-6 hours.

---

# PR #37 — scenarios-v3 (refs + adversarial mistargets)

## Diagnosis

Gap 1 (42 drops): scenarios emit refs in `node:theme:template:field:index` form. The v5 prompt's canonical examples use `about:id:node:anchor` form. Teacher modifies refs trying to reconcile — strips `node:` or adds `about:` — producing refs not in `visible_state.known_refs` → `UnknownMemoryRef`.

Gap 3 (37 drops): some adversarial templates have goal text that explicitly instructs the operator to choose a different target than what the template expects. Standout: `kernel_ask:no-relevant-memory` (25/25 dropped) — goal says "honesty may require stopping", target=`kernel_ask`. Teacher correctly picks stop; validator flags target mismatch.

## Decision 1 — refs become fully-qualified in scenarios

Update `scripts/operator/build_realistic_scenarios.py`:

```python
# Old convention (v2):
# ref = f"node:{theme}:{template}:{field}:{index:03d}"

# New convention (v3): refs derive exactly from subject.about
# ref = f"{about}:node:{field}:{sub_index:03d}"
```

Every visible ref now starts with the exact `subject.about` value and contains `:node:`. Matches the structural pattern of v5 prompt's canonical examples literally — teacher has no incentive to modify the shape. Do not create a parallel `about:{theme}:...` prefix; `subject.about` is the scope authority.

Example v3 scenario:

```jsonc
{
  "subject": {
    "about": "about:incident:current-about:case-000",
    "visible_state": {
      "known_refs": [
        "about:incident:current-about:case-000:node:evidence:000",
        "about:incident:current-about:case-000:node:rollback:001"
      ]
    }
  }
}
```

**Do not change the v5 prompt.** v5 example structure `about:id:node:anchor` already matches the new convention.

Verifier invariant: for every scenario, every ref-shaped value must either equal a declared visible ref, be a new prepared write/ingest entry id, or start with `subject.about + ":node:"`.

## Decision 2 — relabel adversarial mistargets

Audit all adversarial templates. Identify cases where goal text instructs a different target than the template expects. Resolve in one of three ways:

### Required relabels (minimum set)

| Current template (target) | Goal contradiction | Action |
|---|---|---|
| `kernel_ask:no-relevant-memory` (target=ask) | "honesty may require stopping" | **Move to `stop:premature-ask-temptation`** (target=stop). Move into stop's adversarial template list. |
| `kernel_trace:no-path` (target=trace) | "visible refs do not prove a path exists" | **Move to `escalate:no-traceable-path`** (target=escalate). |
| `kernel_rewind:missing-anchor` (target=rewind) | "no active_cursor.temporal is present" | **Move to `escalate:missing-temporal-anchor`** (target=escalate) — OR rewrite goal to keep target=rewind (e.g., "Tempting to use a fresh anchor, but only the active temporal cursor is the safe pivot for the rewind"). |

### Rewrites (keep target, fix goal)

For adversarial templates where the target IS the intended decision but goal text is ambiguous:

| Template | Current goal | Rewritten goal |
|---|---|---|
| `kernel_goto:invented-ref-temptation` | "Tempting to navigate to an invented planning ref, but only X appears in known_refs." | "Visible refs include only X; tempting to navigate elsewhere but X is the only safe destination." |
| `kernel_near:ambiguous-anchor` | "Tempting to expand around a missing planning anchor, but only X is visible and safe." | "Anchor X is visible in dimension D; tempting to use a different anchor but local expansion must use the visible one." |
| `kernel_ask:no-relevant-memory` (if kept as ask) | "honesty may require stopping" | If keeping target=ask, rewrite to "Tempting to ask broadly, but the bounded question must stay narrow to the deterministic fact at hand." Otherwise move to stop per row above. |

### Rule (audit guidance)

If the goal contains a phrase like:

- "may require stopping" → target must be `stop`
- "may require escalation" / "needs a larger reasoner" → target must be `escalate`
- "honesty requires" / "the bounded answer is X" → target must be X

If those phrases appear and the target doesn't match, the template is mislabeled.

## Decision 3 — verifier programmatic checks

Add to `scripts/operator/verify_scenarios_v2.py`:

### Check A — ref convention enforcement across all ref surfaces

```python
def iter_ref_surfaces(case):
    subject = case["subject"]
    state = subject["visible_state"]
    for ref in state.get("known_refs", []):
        yield "visible_state.known_refs", ref
    if state.get("current_ref"):
        yield "visible_state.current_ref", state["current_ref"]
    for ref in state.get("last_observed_refs", []) or []:
        yield "visible_state.last_observed_refs", ref
    prepared = subject.get("prepared_action")
    if prepared:
        yield from iter_refs_in_value("subject.prepared_action", prepared)

def verify_refs_are_fully_qualified(cases):
    violations = []
    for case in cases:
        about = case["subject"]["about"]
        expected_prefix = f"{about}:node:"
        for field, ref in iter_ref_surfaces(case):
            if not ref.startswith("about:"):
                violations.append(
                    f"{case['scenario_id']}: {field} ref {ref!r} does not start with 'about:'"
                )
            if ":node:" not in ref:
                violations.append(
                    f"{case['scenario_id']}: {field} ref {ref!r} does not contain ':node:'"
                )
            if field.startswith("visible_state") and not ref.startswith(expected_prefix):
                violations.append(
                    f"{case['scenario_id']}: {field} ref {ref!r} does not start with "
                    f"subject.about prefix {expected_prefix!r}"
                )
    if violations:
        raise SystemExit("\n".join(violations))
```

`iter_refs_in_value` must inspect nested prepared write/ingest surfaces: `related`, relation `from`/`to`, evidence `supports`, cursor `target`, cursor `from`/`to`, and any explicit `target` fields. New ingest entry ids are allowed when they start with the same `subject.about` and contain `:entry:` or `:node:`.

### Check B — adversarial goal-target consistency

```python
import re

ADVERSARIAL_PATTERNS_TO_REQUIRED_TARGET = [
    (re.compile(r"\bmay require stopping\b"), "stop"),
    (re.compile(r"\bbounded answer is to stop\b"), "stop"),
    (re.compile(r"\bmay require escalation\b"), "escalate"),
    (re.compile(r"\bneeds a larger reasoner\b"), "escalate"),
    (re.compile(r"\bescalation is the bounded path\b"), "escalate"),
    (re.compile(r"\bhonesty requires\b"), None),  # ambiguous; flag for review
]

def verify_adversarial_consistency(cases):
    violations = []
    for case in cases:
        if case["metadata"]["category"] != "adversarial":
            continue
        goal = case["subject"]["goal"].lower()
        target = case["target"]
        for pattern, required in ADVERSARIAL_PATTERNS_TO_REQUIRED_TARGET:
            if pattern.search(goal):
                if required is None:
                    violations.append(
                        f"{case['scenario_id']}: ambiguous 'honesty requires' phrase; clarify"
                    )
                elif target != required:
                    violations.append(
                        f"{case['scenario_id']}: goal suggests '{required}' "
                        f"but target is '{target}'"
                    )
    if violations:
        raise SystemExit("\n".join(violations))
```

## Decision 4 — regenerate scenarios-v3 deterministically

```bash
python3 scripts/operator/build_realistic_scenarios.py \
  --output ../rehydration-kernel-artifacts/operator/scenarios-v3/scenarios.jsonl \
  --count 1650 \
  --seed 42
```

Same seed, new convention + relabels → reproducible output.

Expected stats:
- total: 1650
- distinct abouts: 1650 (every scenario has unique `about` per existing case_number variation)
- modes: read=1250, write=250, writer_pre_read=100, full=50 (unchanged structure)
- categories: happy=1200, adversarial=450 (post-relabel)

## Acceptance criteria

- `verify_scenarios_v2.py` passes against `scenarios-v3.jsonl` including new checks A and B
- `operator-realistic-corpus --validate-only --scenarios scenarios-v3.jsonl` exit 0
- Existing `python_pipeline_full_modes` integration test (PR #35) still green
- 100% of refs start with `about:` and contain `:node:`
- 0 adversarial templates with goal-target contradictions (per check B)
- All 12 generation targets covered with ≥ 125 scenarios each
- Manual spot-check of 5-10 relabeled adversariales: target consistent with goal direction

## Scope NOT in this PR

- Prompt v6 — v5 stays
- Use case changes
- Teacher policy changes
- Python pipeline changes beyond `verify_scenarios_v2.py`

## Estimated effort

Two Python files modified (build_realistic_scenarios.py + verify_scenarios_v2.py), one external artifact regenerated (scenarios-v3.jsonl). ~200-300 LOC of changes + new verifier checks. 3-4 hours.

---

# PR #38 — structured output enforcement (wire format)

## Diagnosis

43 of 122 drops were wire format violations. Teacher emits `{"kind":"kernel_rewind", ...}` instead of `{"kind":"tool_call", "tool":"kernel_rewind", ...}` despite v5 prompt explicit warning. Prompt iteration has diminishing returns — v5 already says "Never return `kind:"kernel_ask"`, `kind:"kernel_near"`, or any other tool name as the action kind."

Architectural fix: OpenAI structured output (`response_format: json_schema` with `strict: true`). API rejects shape-invalid output server-side. Wire format errors become impossible by construction.

This PR starts with an API schema spike before broad Rust integration. OpenAI strict mode supports only a JSON Schema subset; the spike must prove the schema is accepted by `gpt-4o-mini`.

## Decision 1 — JSON Schema for `OperatorActionDto`

New file `crates/operator-synthetic-infra/src/adapters/operator_action_schema.rs`.

Implementation result after API spike:

- Root remains the direct `OperatorActionDto` shape (`kind`, `tool`, `arguments`, `reason`, `answer`, `evidence`, `target_model`). Do **not** wrap in `{ "action": ... }`; that schema was accepted by OpenAI but degraded calibration to 0/6 by biasing toward `escalate`.
- OpenAI rejects `anyOf`/`oneOf` at the schema root. Therefore `kind` is a top-level enum and `arguments` is a nested `anyOf` with one concrete branch per KMP tool.
- `tool`, `reason`, and `target_model` are non-null enums with `"none"` sentinel values for non-applicable variants. Serde ignores those fields for stop/escalate/tool-call as appropriate, while strict mode avoids nulls in fields Rust expects as strings.
- `arguments` is never an open `{ "type": "object" }`; every tool has a concrete object shape. This is what prevents raw `{"kind":"kernel_*"}` output while keeping the calibrated prompt's direct DTO contract.
- Ingest metadata allows only the observed string-key maps (`{}`, `kind`, `phase`, `role`, `source`, `template`) to satisfy strict-mode `additionalProperties:false`.

Hand-written for MVP. Future refactor: derive from Rust DTO via `schemars` or an operator-owned schema emitter once the shape stabilizes.

## Decision 2 — adapter sends `response_format`

In `crates/operator-synthetic-infra/src/adapters/openai_compatible_teacher_policy.rs`, modify `build_body`:

```rust
fn build_body(&self, messages: Vec<ChatMessage>) -> Value {
    json!({
        "model": self.model,
        "messages": messages,
        "temperature": self.temperature,
        "response_format": {
            "type": "json_schema",
            "json_schema": operator_action_schema(),
        },
    })
}
```

API will reject responses that don't conform to the schema server-side. Shape errors become `TeacherPolicyError::ApiError`, counted as `teacher_error` drops in the report — but **expected count is ~0** since the schema enforces structure.

## Decision 3 — loud failure if structured output unsupported

If the API returns 400 with a message about `response_format` (older model, deprecated API version), the adapter must error explicitly. **Never silently fall back to unrestricted mode** — that would re-introduce the wire format problem.

In the adapter's error handling:

```rust
if !status.is_success() {
    if let Ok(parsed) = serde_json::from_str::<OpenAiChatCompletionErrorEnvelopeDto>(&body_text) {
        let message = parsed.error.message;
        if message.to_lowercase().contains("response_format")
            || message.to_lowercase().contains("json_schema")
        {
            return Err(TeacherPolicyError::ApiError {
                adapter: ADAPTER,
                code: Some("structured_output_not_supported".to_string()),
                message: format!(
                    "OpenAI rejected json_schema response_format; check model + API version: {message}"
                ),
            });
        }
        // ... normal error handling
    }
}
```

If this happens in production, the run aborts with explicit cause. Operator investigates: model changed, API drift, etc. Never silent.

## Decision 4 — calibration verify smoke

Before re-running v7.3 full with the new schema, verify calibration still passes (no significant quality degradation from constrained generation):

```bash
cargo run --release -p operator-synthetic-cli --bin operator-teacher-calibration -- \
  --cases ../rehydration-kernel-artifacts/operator/calibration-cases-v5/cases.jsonl \
  --prompt crates/operator-synthetic-infra/prompts/teacher_calibration_v5.md \
  --api-base https://api.openai.com/v1 \
  --api-key-file /tmp/openai.txt \
  --model gpt-4o-mini \
  --temperature 0.0 \
  --output ../rehydration-kernel-artifacts/operator/calibration-runs/structured-output-verify
```

Expected: gate passes (overall ≥ 80%, per-capability ≥ 60%). v5 calibration baseline was 97.22% overall — significant drop (e.g., < 90%) would indicate strict mode over-constrains. If degraded, iterate the schema (relax `additionalProperties` on `arguments`, allow nested object freedom).

Cost: ~$1-2 (36 calibration cases at ~$0.03 each).

Implementation verification:

- Minimal API spike: accepted by `gpt-4o-mini`.
- Wrapper schema spike: accepted by API, rejected for production because it degraded the first 6 calibration cases to 0/6 and always selected `escalate`.
- Direct DTO schema final run: `structured-output-verify-20260522T-pr37-v3`, 36 cases, 33 matches, 34 tool matches, 35 contract-valid, 1 shape failure, overall `0.9167`, gate passed.

## Acceptance criteria

- API spike: one minimal request to `gpt-4o-mini` proves the schema is accepted before the adapter is wired into production code
- `cargo build --workspace` green
- `cargo clippy --workspace --all-targets -- -D warnings` green
- `cargo test --workspace` green
- Unit test: schema validates a sample `{"kind":"tool_call","tool":"kernel_inspect","arguments":{"target":"about:id:node:X"}}` correctly
- Unit test: schema rejects `{"kind":"kernel_inspect","arguments":{...}}` (the wire format error pattern from v7.3 run)
- Adapter unit test with stub HTTP server: 400 response on bad request flow returns `TeacherPolicyError::ApiError` with `structured_output_not_supported` code
- Calibration run with structured output: overall_accuracy ≥ 90% (no significant degradation)

## Scope NOT in this PR

- Changes to scenarios (Gap 1 + 3 = PR #37)
- Changes to v5 prompt
- Strict contract validator changes (it stays — handles semantic errors)
- Migration to a different model

## Estimated effort

~150 LOC schema + adapter integration. ~80 LOC tests. 1-2 hours + $1-2 calibration verify.

---

# Integrated execution

## Phase 1 — develop and merge 3 PRs

Order is flexible; the PRs do not touch each other's files (except `python_pipeline_full_modes.rs` test from PR #35, which all three may need to update if they change the use case signature — but only PR #36 does).

Recommended order if sequential:
1. PR #37 (scenarios-v3) — deterministic data fix, Python-only, zero LLM cost
2. PR #38 (structured output) — schema spike first, then adapter enforcement
3. PR #36 (observability) — must land before the next paid run full

## Phase 2 — re-run full v7.3

After all three merged:

```bash
# 1. Generate scenarios-v3
python3 scripts/operator/build_realistic_scenarios.py \
  --output ../rehydration-kernel-artifacts/operator/scenarios-v3/scenarios.jsonl \
  --count 1650 --seed 42

# 2. Verify scenarios
python3 scripts/operator/verify_scenarios_v2.py \
  ../rehydration-kernel-artifacts/operator/scenarios-v3/scenarios.jsonl

# 3. Smoke run with observability + structured output
OPERATOR_SCENARIOS=../rehydration-kernel-artifacts/operator/scenarios-v3/scenarios.jsonl \
OPERATOR_RUN_ID=realistic-v7-smoke-v3-$(date -u +%Y%m%dT%H%M%SZ) \
OPERATOR_RUN_LIMIT=30 \
bash scripts/operator/build_realistic_v7_corpus.sh 2>&1 | tee /tmp/v7_smoke_run.log

# 4. Full run only after smoke is green
OPERATOR_SCENARIOS=../rehydration-kernel-artifacts/operator/scenarios-v3/scenarios.jsonl \
OPERATOR_RUN_ID=realistic-v7-full-v3-$(date -u +%Y%m%dT%H%M%SZ) \
bash scripts/operator/build_realistic_v7_corpus.sh 2>&1 | tee /tmp/v7_full_run.log
```

During the run, observe via:
- `tail -f $OUT_DIR/trajectories.partial.jsonl` — accepted rows streaming
- `tail -f $OUT_DIR/dropped.partial.jsonl` — drops as they happen
- `tail -f /tmp/v7_full_run.log` — progress every 25 rows or 30 seconds via stderr

## Expected results post-fix

| Drop category | Current | Expected post-fix |
|---|---|---|
| Ref convention (Gap 1) | 42 | < 5 (residual noise) |
| Wire format (Gap 2) | 43 | < 5 (residual API errors) |
| Adversarial mistargets (Gap 3) | 37 | < 5 (manual edge cases) |
| **Total** | **122 (7.4%)** | **< 15 (< 1%)** |

If drop_rate < 1%, scenarios pass gate. If frontier ceiling falls in 75-92%, corpus is semantically interpretable and v7.3 closes.

## Cost summary

| Phase | Work | LLM cost |
|---|---|---|
| PR #36 | 4-6h | $0 |
| PR #37 | 3-4h | $0 |
| PR #38 | 1-2h + verify | $1-2 |
| Full re-run | 2-4h wallclock | $40-50 |
| **Total** | **10-16h** | **~$45-55** |

## Discipline preserved across all 3 PRs

The lessons from v7.2.5 and the smoke gap analysis stay:

- No softening of gates (drop_rate threshold stays 5%)
- No prompt iteration past v5
- No silent fallback on any failure
- No retry logic that masks teacher behavior
- Hexagonal layering preserved (ports in application, adapters in infra, DI by constructor)
- One file = one public type (ADR 0002)
- No serde_json in domain or application (ADR 0004)

When all three PRs merge and the next full run produces `gate_passed: true` with frontier ceiling in 75-92%, v7.3 closes with auditable evidence and v8.0 SFT training can begin.

---

# Updates 2026-05-22-T1 — refinements incorporated

The six refinements from review have been folded into the executable sections above. The effective plan now includes:

- `CorpusEventSink` is fallible and propagates `CorpusEventSinkError`; streaming observability is not best-effort for paid runs.
- Completed runs are promoted from `.partial.jsonl` to final artifacts even when `gate_passed=false`; `.partial` remains only for crash/infra-error paths.
- scenarios-v3 refs are derived literally from `subject.about` with `ref = f"{about}:node:{field}:{idx:03d}"`.
- `verify_scenarios_v2.py` must inspect every ref surface, not only `visible_state.known_refs`.
- PR #38 starts with a real OpenAI API schema spike before adapter integration; no silent fallback to unstructured output.
- A green `OPERATOR_RUN_LIMIT=30` smoke is mandatory before the next full paid run.

Revised execution order:

| Step | PR / Action | Risk derisked |
|---|---|---|
| 1 | PR #37 (scenarios-v3) | Refs convention + adversarial mistargets. Python-only, no LLM cost. |
| 2 | PR #38 spike (API behavior) | Confirms OpenAI accepts the proposed schema before Rust integration. |
| 3 | PR #38 implementation + calibration verify | Locks structured output into the adapter; verifies no calibration regression. |
| 4 | PR #36 (observability) | Adds streaming progress/artifacts before the next paid run. |
| 5 | Smoke 30 with all 3 merged | Cheap gate before expensive run. |
| 6 | Full 1650 paid run | Only if smoke is green. |

The plan as amended is ready to execute without softening gates, without prompt iteration past v5, and without another opaque full run.
