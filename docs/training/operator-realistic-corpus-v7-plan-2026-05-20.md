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

- 3-5 episodes;
- all tool categories touched;
- unit tests for parsing, split policy and corpus-quality specs.

This is not for training. It is for architecture and gate validation.

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

## Non-goals

This corpus is not:

- a MemoryArena or LongMemEval adapter;
- a benchmark solver;
- a replacement for replay against real KMP;
- a place to teach final-answer reasoning;
- a place to teach the 0.5B model to invent graph relations.

Benchmark adapters remain in `rehydration-kernel`. Operator consumes
trajectory-shaped data and learns bounded KMP/MCP operation.
