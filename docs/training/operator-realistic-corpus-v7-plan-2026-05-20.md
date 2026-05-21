# Operator realistic corpus v7 plan — 2026-05-20

## Decision

Do not train a release-candidate Operator model on the contract-v6 fixture
corpus.

The v6 corpus is valid and useful as a pipeline gate:

- 10/10 KMP/MCP tool coverage;
- strict `kind` contract;
- train/eval SFT round trip;
- no-gold audit;
- validate-only trainer/predictor checks.

It is not realistic training data. Training on it would produce an inflated
score because the generator creates repeated canonical fixtures, not real
operator decision trajectories.

The next interpretable training run must wait for a realistic corpus. This
document defines that corpus.

## Goal

Teach a small Operator model to choose the next bounded KMP/MCP action during a
real process.

The target behavior is not "answer the user". The target behavior is:

```text
given visible KMP state + objective + budget + allowed tools
choose the next valid operator action
```

The model must learn when to:

- inspect;
- move near a node;
- move forward or backward in time;
- trace a path;
- continue a paginated trace;
- stop because enough evidence is visible;
- escalate because the decision needs a larger reasoning model;
- write memory using a prepared smart write;
- execute canonical ingest when the payload is already prepared and valid.

## Dataset Unit

The atomic row remains `TrainingTrajectoryDto`.

v7 changes the trajectory contract: `TrainingTrajectory` and
`TrainingTrajectoryDto` must carry an explicit `goal: NonEmptyString`.
`task_family` stays as a structured classification for metrics and splitting;
it is not the operator objective. The goal must be persisted with the row so
post-hoc audits can explain why the target action was correct.

The realistic unit is an episode. An episode is a coherent process around one
`about`, such as an incident, investigation, migration, or product decision.
Each episode produces multiple trajectory rows.

```text
episode
  -> step 1: visible_state + goal -> target_action
  -> step 2: visible_state + goal -> target_action
  -> ...
```

Train/eval splits must happen by episode or `about`, never by random row. Row
splits can leak the same process into both train and eval and inflate scores.

## Episode Themes

Start with non-medical, process-heavy domains:

| Theme | Why it is useful |
| --- | --- |
| technical incident | hypotheses, false paths, multi-agent investigation, rollback/fix decisions |
| bug investigation | logs, versions, symptoms, root cause, contradicted assumptions |
| software migration | constraints, staged decisions, superseded plans, temporal changes |
| product planning | conflicting requirements, preference changes, final decision rationale |
| benchmark-like memory task | multi-session facts, preferences, updates, temporal disambiguation |
| smart writing session | pre-read context, relation choice, rich relation vs anemic fallback |

The corpus should use several domains from the first cut. A single theme is
too easy to overfit.

## Required Capabilities

The corpus must cover all public KMP/MCP actions in realistic situations:

| Capability | Required realistic situations |
| --- | --- |
| `kernel_wake` | open a bounded current `about` before deeper navigation |
| `kernel_ask` | retrieve deterministic evidence without asking for generative reasoning |
| `kernel_near` | expand around a candidate node with explicit limit/window pressure |
| `kernel_goto` | jump to a known ref, temporal cursor, or trace target |
| `kernel_rewind` | move to prior state when a later node depends on hidden earlier context |
| `kernel_forward` | move forward after a rewind/goto to test what changed later |
| `kernel_trace` | reconstruct why one node replaced, contradicted, or depended on another |
| `kernel_inspect` | read node detail when summary/evidence refs are insufficient |
| `kernel_write_memory` | execute a prepared smart write with relation proof |
| `kernel_ingest` | execute canonical write when the full typed payload is already prepared |
| `stop` | stop when evidence is sufficient and more calls would only add cost |
| `escalate` | escalate when the next decision requires semantic inference outside Operator |

The corpus also needs cross-cutting coverage:

- current `about`, selected `abouts`, and all-about scope cases;
- pagination first page and continue page;
- bounded budgets and low-budget stop/escalate decisions;
- explicit page metadata handling;
- read-before-write proof;
- relation quality decisions;
- negative cases where a tempting action is invalid because the ref, cursor,
  dimension, scope, or payload is not visible.

## Writer Behavior

Writing is the highest-risk area. The corpus must not teach Operator to invent
meaning.

For smart writing, the writer/teacher first reads the graph:

```text
wake/ask/near/trace/inspect
  -> decide whether enough context is visible
  -> choose relation only if supported
  -> write or stop/escalate
```

Valid write rows must prove:

- the target `about` is the trajectory `about`;
- every relation endpoint is visible or part of the new ingest;
- rich relations carry evidence and why;
- relation quality is intentional;
- anemic fallback is used when no honest richer relation is visible;
- unsupported/vague relations are rejected before reaching KMP.

The 0.5B Operator should not be trained to invent relation semantics. It should
learn to choose the safe next action:

- call another KMP read tool;
- execute a prepared write;
- use an anemic fallback when explicitly prepared;
- stop/escalate when the relation cannot be justified.

## Teacher Generation

The teacher should be a strong model or deterministic generator plus a strong
model, but its output is never trusted directly.

The generation loop:

1. Build an episode spec with domain facts, dimensions, timeline, relations,
   hidden distractors, and expected operator objectives.
2. Populate a real KMP server through the public KMP/MCP write contract.
3. Let the teacher operate only through the public KMP/MCP tool contract.
4. Capture every decision as `TrainingTrajectoryDto`.
5. Validate every row through the strict Operator contract.
6. Replay selected trajectories against KMP/MCP.
7. Reject rows that repair, infer, or reference invisible state.

The teacher can reason. The corpus must still be deterministic, inspectable and
contract-valid.

Selected population mode:

```text
real KMP + memory seeded by teacher/deterministic builder via kernel_ingest
```

Manual hand-populated KMP is allowed only for calibration fixtures. Mock kernels
are allowed for unit tests and architecture smokes, not for training data.

## Validation Gates

A realistic corpus run is acceptable only if all gates pass:

```text
schema parse                      every row parses as TrainingTrajectoryDto
action parse                      every target parses as OperatorActionDto
contract coverage                 all KMP/MCP tools covered
mode safety                       every tool allowed in row mode
reference safety                  target refs/cursors/dimensions are visible
scope safety                      about/scope choices are explicit
pagination safety                 trace/navigation page metadata handled
write proof                       rich writes have read-before-write evidence
no-gold audit                     model-facing prompts do not leak labels
episode split                     train/eval separated by episode/about
duplicate audit                   model-facing duplicate rows bounded/reported
replay smoke                      selected rows execute against KMP/MCP
frontier ceiling                  mandatory frontier baseline measured
```

Any failure stops the run. No fallback, no row patching.

## Metrics

Do not rely only on exact-action accuracy.

Required report dimensions:

| Metric | Meaning |
| --- | --- |
| `exact_action_accuracy` | tool and arguments match exactly |
| `tool_selection_accuracy` | correct KMP/MCP action chosen |
| `argument_validity_rate` | action parses and passes strict specs |
| `contract_valid_rate` | strict Operator contract pass rate |
| `stop_correctness` | stops only when evidence is sufficient |
| `escalation_correctness` | escalates only when Operator should not decide |
| `pagination_correctness` | first/continue page decisions are correct |
| `scope_safety_rate` | no accidental global or cross-about read/write |
| `write_relation_quality` | rich/anemic/suspect relation distribution |
| `read_before_write_proof` | write decisions backed by visible read context |
| `budget_behavior` | expands/shrinks/stops under call/token pressure |

The v4 24.1% conformance baseline remains the comparison anchor until a
realistic strict corpus produces a new baseline.

## Calibration

Before generating a full corpus, run the teacher through a handcrafted
calibration set:

```text
calibration episodes: 20-30
known-correct decisions: yes
minimum teacher score: 80%
```

This is policy quality control, not architecture validation. If the teacher
scores below 80%, fix the teacher prompt/policy before spending money on a full
corpus.

## First Target Size

Use two size classes:

```text
v7 training smoke:
  episodes:          25-40
  decisions/episode: 8-20
  rows:              300-600
  purpose:           pipeline + learning-signal smoke only

v7 interpretable training baseline:
  rows:              1500-3000 minimum
  eval:              20-30% by episode/about
  frontier ceiling:  mandatory
```

The 300-600 row corpus is not enough for a publication/release-candidate claim.
It can show whether the pipeline works and whether the model starts learning.
For an interpretable baseline, use at least 1500-3000 realistic rows or the
result stays ambiguous.

## Artifact Layout

Generated artifacts should stay outside the repo:

```text
../rehydration-kernel-artifacts/operator/realistic-v7-<run-id>/
  episode_specs/
  trajectories.jsonl
  sft/
    openai_train.jsonl
    openai_eval.jsonl
    train_trajectories.jsonl
    eval_trajectories.jsonl
    summary.json
  coverage.json
  replay_report.json
  no_gold_audit.json
  frontier_ceiling/
    predictions.jsonl
    summary.json
    policy_eval.json
  generation_log.jsonl
```

The repo should contain only:

- generator code;
- scenario templates;
- runbooks;
- tests;
- small fixtures needed for unit/integration tests.

## Implementation Slices

### v7.0 — design and gates

Status: this document.

Lock the decision that v6 is a contract gate only and define the realistic
corpus acceptance criteria.

### v7.1 — episode domain model

Add typed synthetic-domain concepts:

- `SyntheticEpisodeSpec`;
- `EpisodeTheme`;
- `EpisodeObjective`;
- `EpisodeStepPlan`;
- `CapabilityTarget`;
- `EpisodeSplitPolicy`.

Add the corpus-quality model without introducing a `CorpusQualityGate` code
type:

- one `Specification<CorpusSnapshot>` per quality rule;
- `CompositeCorpusQualityValidator` as the domain composite;
- `EvaluateCorpusQualityUseCase` as the application orchestrator;
- `CorpusSource` as the application port.

Also add the explicit row objective:

- `TrainingTrajectory.goal: NonEmptyString`;
- `TrainingTrajectoryDto.goal`;
- mapper updates;
- SFT prompt projection from the persisted goal.

No JSON in domain/application. DTOs and mappers live at the boundary if
episode specs need to be persisted.

### v7.2 — seed episode fixtures

Create a small hand-authored fixture set:

- exactly 5 episodes;
- all KMP/MCP tools plus `stop` and `escalate` touched;
- unit tests for parsing, split policy and corpus-quality specs.

This is not for training. It is for architecture and gate validation.

The v7.2 fixture set is fixed:

| Episode | Theme | Dominant mode | Required actions |
| --- | --- | --- | --- |
| `episode_incident_payments_timeout` | technical incident | read -> stop | `kernel_wake`, `kernel_ask`, `kernel_near`, `kernel_trace`, `kernel_inspect`, `kernel_rewind`, `stop(answer_ready)` |
| `episode_software_migration` | software migration | read -> write -> stop | `kernel_wake`, `kernel_ask`, `kernel_near`, `kernel_forward`, `kernel_inspect`, `kernel_ingest`, `stop(answer_ready)` |
| `episode_bug_investigation` | bug investigation | read -> escalate | `kernel_wake`, `kernel_ask`, `kernel_trace`, `kernel_inspect`, `kernel_goto`, `kernel_rewind`, `escalate` |
| `episode_product_planning` | product planning | read -> write -> stop | `kernel_wake`, `kernel_ask`, `kernel_near`, `kernel_trace`, `kernel_inspect`, `kernel_write_memory`, `stop(answer_ready)` |
| `episode_smart_writing` | smart writing session | read -> write -> stop | `kernel_wake`, `kernel_ask`, `kernel_near`, `kernel_inspect`, `kernel_ingest`, `stop(answer_ready)` |

The combined seed corpus must additionally cover:

- `kernel_forward` at least once;
- `kernel_rewind` in at least two episodes;
- paginated trace state through an active cursor;
- budget pressure on at least one stop/escalate row;
- exactly one rich `kernel_ingest` relation with explicit `why` and evidence;
- exactly one anemic fallback relation;
- exactly one `escalate`.

The spec fixture matrix is:

| Spec | Failing fixture | Expected code |
| --- | --- | --- |
| `schema_parse_spec` | `corpus_with_unparseable_row()` | `SchemaParse` |
| `action_parse_spec` | `corpus_with_invalid_action_target()` | `ActionParse` |
| `contract_coverage_spec` | `corpus_missing_kernel_forward()` | `ContractCoverage` |
| `mode_safety_spec` | `corpus_with_ingest_in_read_mode()` | `ModeSafety` |
| `reference_safety_spec` | `corpus_with_unknown_memory_ref()` | `ReferenceSafety` |
| `scope_safety_spec` | `corpus_with_about_not_in_known()` | `ScopeSafety` |
| `pagination_safety_spec` | `corpus_with_trace_lacking_cursor_continuation()` | `PaginationSafety` |
| `write_proof_spec` | `corpus_with_write_lacking_read_before_write()` | `WriteProof` |
| `no_gold_audit_spec` | `corpus_with_target_action_leaked_in_system_prompt()` | `NoGoldAudit` |
| `episode_split_spec` | `corpus_with_train_and_eval_sharing_about()` | `EpisodeSplit` |
| `duplicate_audit_spec` | `corpus_with_duplicate_model_facing_rows()` | `DuplicateAudit` |
| `replay_smoke_spec` | `corpus_with_action_failing_mcp_request_shape()` | `ReplaySmoke` |
| `frontier_ceiling_spec` | `corpus_without_frontier_baseline_recorded()` | `FrontierCeiling` |

v7.2 validates the architecture. It is not training data, not calibration
data, and not a teacher-policy measurement.

### v7.2.5 — teacher calibration

Create 20-30 handcrafted calibration episodes with known-correct operator
decisions. The teacher must pass at least 80% before v7.3 generates a larger
corpus.

Calibration rows are audit fixtures. They can be committed if small and
sanitized. They are not the training corpus.

### v7.3 — teacher-backed generator adapter

Add an infra adapter that can call a teacher model through an explicit port.
Reuse existing LLM infrastructure instead of introducing a second client stack:

- `LlmBaseliner` port;
- `OpenAiCompatibleLlmBaseliner` adapter;
- `ChatMessage` VO;
- OpenAI-compatible HTTP flow already used by `operator-llm-baseline`;
- existing `operator-contract-coverage`;
- existing `audit_operator_sft_no_gold.py`;
- existing `round_trip_smoke.sh` pattern.

The new work is scenario building, teacher policy, episode loop, and
capability-specific metrics. It is not a new LLM client project.

The teacher output must be validated into domain objects before it becomes a
trajectory. Invalid teacher output is rejected, not repaired.

### v7.4 — realistic corpus dry run

Generate the first 25-40 episode corpus. Run:

- contract coverage;
- no-gold audit;
- train/eval episode split audit;
- duplicate audit;
- replay sample;
- mandatory frontier ceiling on the eval split.

### v7.5 — 0.5B training smoke

Only after v7.4 gates pass:

- train Qwen 0.5B LoRA;
- predict on episode-held-out eval;
- score exact/tool/contract plus capability metrics;
- compare against the frontier ceiling;
- document the run in `docs/training/runs/`.

If this run uses only 300-600 rows, label it as a training smoke. Do not use it
as a release-candidate model-history result.

### v7.6 — interpretable training baseline

Only after the smoke proves the pipeline:

- scale to 1500-3000 realistic rows minimum;
- keep episode/about split;
- run the mandatory frontier ceiling first;
- train Qwen 0.5B LoRA;
- compare against both the v4 24.1% anchor and the v7 frontier ceiling;
- publish model-history only if the result is interpretable.

## Updates 2026-05-20-T7

The following gaps were closed before starting v7.1:

| Gap | Decision |
| --- | --- |
| kernel population | use real KMP seeded through `kernel_ingest`; mocks are only for tests |
| row objective | add `goal: NonEmptyString` to `TrainingTrajectory` and DTO in v7.1 |
| teacher calibration | require 20-30 handcrafted calibration episodes and >=80% teacher score |
| sample size | 300-600 rows is a training smoke; 1500-3000 rows minimum for interpretable baseline |
| frontier ceiling | mandatory on the v7 eval split |
| infrastructure reuse | reuse existing LLM baseline/client, coverage, no-gold audit and smoke tooling |

This prevents v7.3 from silently choosing a mock kernel, silently omitting the
objective, or reinventing the existing OpenAI-compatible LLM client path.

## Updates 2026-05-21-T8

v7.1 implementation checklist before review:

| Area | Files to verify | Requirement |
| --- | --- | --- |
| shared domain | `operator-shared-domain/src/value_objects/trajectory_goal.rs`, `operator-shared-domain/src/trajectory/training_trajectory.rs` | `TrajectoryGoal` is a non-empty value object and `TrainingTrajectory` requires it |
| shared contract | `operator-shared-contract/src/training_trajectory_dto.rs` | `goal` is a required wire field; old rows without it are invalid |
| shared infra | `operator-shared-infra/src/mappers/training_trajectory_mapper.rs` | DTO/domain mapping preserves `goal` both ways |
| synthetic generation | `operator-synthetic-infra/src/generators/in_memory_synthetic_case_generator.rs` | generated contract rows carry a visible objective |
| SFT preparation | `scripts/operator/prepare_operator_sft_dataset.py` | raw trajectories without non-empty `goal` fail before prompt construction |
| SFT validation/prediction | `scripts/operator/predict_operator_sft.py`, `scripts/operator/train_operator_sft_lora.py` | model-facing user payloads require non-empty `goal` |
| smoke fixtures | `operator-*-cli/tests/*.rs` and shared infra tests | every hand-written JSONL trajectory includes `goal` |
| documentation | shared architecture docs | `TaskFamily` is taxonomy; `TrajectoryGoal` is the row objective |

## Updates 2026-05-21-T9

v7.1b implements the synthetic episode and corpus-quality model in the
synthetic bounded context:

- episode aggregate and value objects live in `operator-synthetic-domain/src/episode/`;
- the word "gate" is not a code type; quality rules are individual
  `Specification<CorpusSnapshot>` implementations;
- `CompositeCorpusQualityValidator` composes the 13 corpus-quality specs in
  stable order and accumulates violations;
- `EpisodeSplitPolicy` is only a strategy value object;
- `EpisodeSplitter` is the application service that applies that policy;
- `EvaluateCorpusQualityUseCase` loads a corpus via a port and invokes the
  injected validator.

No infra adapters, DTO mappers, fixtures, teacher policy or training data are
introduced in this slice.

## Updates 2026-05-21-T10

v7.2 fixes the seed-fixture shape:

- exactly five handcrafted episodes;
- approximately 40-50 typed trajectory rows total;
- every row built with domain constructors and validated against the strict
  action contract;
- one clean corpus snapshot shared by all specs;
- one focused failing fixture per corpus-quality spec;
- one composite fixture that fails at least five specs at once.

This is still architecture validation only. It must not be used as training
data or as calibration data for the teacher.

## Updates 2026-05-21-T11

v7.2.5 adds the teacher calibration suite before the v7.3 teacher-backed
generator:

- calibration cases live outside the repo under
  `../rehydration-kernel-artifacts/operator/calibration-cases-v1/cases.jsonl`;
- the committed prompt lives at
  `crates/operator-synthetic-infra/prompts/teacher_calibration_v1.md`;
- the runner is `operator-teacher-calibration`;
- the application owns two ports: `CalibrationEpisodeSource` and
  `TeacherPolicy`;
- infra owns the JSONL source and OpenAI-compatible teacher adapter;
- the teacher receives only the `CalibrationSubject` (`about`, mode,
  task_family, goal, allowed tools and visible KMP state);
- accepted actions and human rationale never cross the LLM boundary;
- reports include overall, per-capability and per-category metrics.

The v1 dataset is intentionally small but policy-focused:

| Property | Value |
| --- | --- |
| cases | 25 |
| category split | 18 happy / 7 adversarial |
| capabilities | 12/12 covered |
| writer-pre-read cases | 3 |
| multi-accepted case | yes |
| contract-valid rows | 25/25 in stub parse check |

The calibration gate requires both:

```text
overall_accuracy >= 0.80
per_capability_accuracy >= 0.60 for every capability
```

Per-category metrics are diagnostic. They do not fail the gate yet, but they
make teacher bias visible: a high happy-case score with weak adversarial
behavior is not acceptable evidence for moving to v7.3.

The calibration flow is:

```text
calibration-cases-vN/cases.jsonl
  -> JsonlCalibrationEpisodeSource
  -> OpenAiCompatibleTeacherPolicy
  -> EvaluateTeacherCalibrationUseCase
  -> report.json
  -> gate pass/fail
```

If the teacher fails, do not edit cases to match the model. Fix the prompt or
teacher policy, rerun, and keep the report as evidence.

### Update 2026-05-21-T9 — first real calibration evidence

The first v7.2.5 manual runs are recorded in
`teacher-calibration-results-2026-05-21.md`.

The important result is not a pass. It is a design finding:

- multi-accepted actions are correct for narrative arguments such as
  `kernel_ask.query` and `stop.answer`;
- structured arguments must remain exact;
- `gpt-4o-mini` is the better teacher candidate observed so far because this
  task rewards literal KMP/MCP argument preservation more than creative
  paraphrasing;
- the current 60% per-capability floor is brittle with only two cases per
  capability: one failure scores 50%, so the floor behaves like a 100% floor;
- `kernel_ingest` is currently the blocking capability;
- the current calibration subject can only carry a narrative `goal`, so prepared
  ingest payloads are being reconstructed from prose;
- that is not the desired long-term Operator responsibility.

Before v7.3 teacher-backed corpus generation, prepared write/ingest payloads
need a typed subject shape or equivalent typed prepared-arguments carrier. The
teacher should decide whether to execute a prepared KMP/MCP action, not learn to
compile long prose into canonical ingest JSON.

The next architectural slice should produce a v4 calibration dataset with at
least three cases per capability, a typed prepared-payload carrier, and a full
`gpt-4o-mini` calibration report with `gate_passed: true`.

PR #32 is therefore infrastructure plus findings. It is not evidence that the
v7.2.5 teacher gate passed.

## Updates 2026-05-21-T12

PR #33 closes the v7.2.5 calibration gate.

Implemented:

- `CalibrationSubject` now has an optional typed `prepared_action`;
- prepared actions are domain values and must be tool calls;
- prepared action tools must be allowed by the subject mode;
- DTO and mapper layers preserve `prepared_action`;
- the CLI stub returns `prepared_action` exactly when present;
- prompt `teacher_calibration_v3.md` tells the teacher to copy a prepared action
  verbatim;
- prompt `teacher_calibration_v4.md` adds an explicit escalation rule for goals
  that say `Escalate with beyond_capability`.
- prompt `teacher_calibration_v5.md` adds the missing canonical
  `kernel_goto` trace-cursor example.

External datasets:

| Dataset | Purpose | Result |
| --- | --- | --- |
| `calibration-cases-v4` | first 36-case prepared-action suite | failed; exposed ambiguous escalation cases |
| `calibration-cases-v5` | corrected 36-case suite, still 3 cases per capability | passed |

Passing report:

```text
../rehydration-kernel-artifacts/operator/calibration-runs/2026-05-21T-pr33-v5-gpt4o-mini-full/report.json
```

Measured result:

| Metric | Value |
| --- | ---: |
| total cases | 36 |
| exact matches | 35 |
| tool matches | 36 |
| contract-valid predictions | 36 |
| shape failures | 0 |
| overall accuracy | 97.22% |
| gate | passed |

This means v7.2.5 is complete for the current scope. v7.3 may start with
`gpt-4o-mini` as the calibrated teacher, subject to the downstream corpus gates.

Prompt v5 was checked once after this gate:

```text
../rehydration-kernel-artifacts/operator/calibration-runs/2026-05-22T-pr33-v5-promptv5-gpt4o-mini-full/report.json
```

It passed the gate and fixed the previous trace-cursor mismatch, but produced
one unrelated `kernel_ask` shape failure. Do not iterate the calibration prompt
again before v7.3; the next signal should come from the generated corpus gates.
A repeat run reproduced the same `kernel_ask` shape failure, so prompt v4
remains the cleanest run-level calibration evidence while prompt v5 documents
the reusable trace-cursor example.

## Updates 2026-05-22-T13

v7.3 has started with the first teacher-backed `SyntheticCaseGenerator`
adapter.

Implemented in this slice:

- `SyntheticGenerationTarget`, a domain target model for 10 KMP tools plus
  `stop` and `escalate`;
- `SyntheticCaseSpec` now stores a generation target instead of assuming every
  case is a KMP capability;
- `SyntheticDatasetBlueprint::for_all_generation_targets` for 12-target corpus
  planning;
- `TeacherBackedSyntheticCaseGenerator<T: TeacherPolicy>` in
  `operator-synthetic-infra`;
- one generated `CalibrationSubject` per requested synthetic row;
- typed prepared actions for `kernel_ingest` and `kernel_write_memory`;
- fail-fast rejection when the teacher chooses the wrong tool;
- fail-fast rejection when the teacher action fails the shared strict action
  contract;
- propagation of teacher policy failures without repair;
- focused tests covering all `SyntheticGenerationTarget` variants.

Important scope boundary:

`KmpMcpCapability` remains the 10-tool KMP/MCP contract. It was not widened to
include non-tool actions. `stop` and `escalate` live in the new synthetic
generation target model, which keeps corpus planning separate from the KMP tool
contract.

The in-memory fixture generator still only supports KMP targets and now rejects
`stop`/`escalate` fail-fast. The teacher-backed generator is the path that can
produce all 12 target kinds.

## Non-goals

This corpus is not:

- a MemoryArena or LongMemEval adapter;
- a benchmark solver;
- a replacement for replay against real KMP;
- a place to teach final-answer reasoning;
- a place to teach the 0.5B model to invent graph relations.

Benchmark adapters remain in `rehydration-kernel`. Operator consumes
trajectory-shaped data and learns bounded KMP/MCP operation.
