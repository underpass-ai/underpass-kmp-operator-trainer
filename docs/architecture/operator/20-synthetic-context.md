# Synthetic Bounded Context

The `synthetic` bounded context owns the **canonical generation of
training trajectories from `KMP`/`MCP` use cases**. It does not score
trajectories, replay them or talk to a real KMP server; those concerns
belong to `evaluation`, `replay` and `runtime` respectively.

The contract with the rest of the system is: given a
[`SyntheticDatasetBlueprint`](shared/domain/README.md), produce a
[`SyntheticDataset`](#dataset-types) of `TrainingTrajectory` values from
the `shared` bounded context, plus a per-case metric so downstream
consumers can enforce coverage.

This pass establishes the skeleton. The in-memory generator covers every
`KmpMcpCapability` with a minimal canonical fixture. The teacher-backed
generator adds an LLM-teacher path over the same port, while the richer
writer/exec scenario library lands in later passes.

The fixture generator is not training-grade. It exists to prove the action
contract, SFT preparation and round-trip pipeline. The realistic training
direction is documented in
[`../../training/operator-realistic-corpus-v7-plan-2026-05-20.md`](../../training/operator-realistic-corpus-v7-plan-2026-05-20.md).

## Crates

```
operator-synthetic-domain    capabilities, episodes, corpus quality specs,
                             case specs, blueprints, reports
operator-synthetic-application use cases, services + generation/corpus ports
operator-synthetic-infra     in-memory adapter; teacher-backed adapter;
                             calibration JSONL/LLM adapters
```

No `operator-synthetic-contract` crate today. See
[decisions/0008-no-synthetic-contract-yet.md](decisions/0008-no-synthetic-contract-yet.md).

## Domain map

### Capability

- `capability/kmp_mcp_capability.rs` — `KmpMcpCapability` closed enum.
  Variants 1:1 with `KernelTool`. Each carries its canonical
  `OperatorMode` (`Read` for read tools, `Write` for `WriteMemory`).
  `ALL` exposes the canonical enumeration.

### Case

- `case/synthetic_case_spec.rs` — `SyntheticCaseSpec` = case_id +
  generation target + minimum_examples.
- `case/synthetic_generation_target.rs` — closed generation target enum:
  all 10 KMP tools plus `stop` and `escalate`.
- `case/synthetic_case_generation_metric.rs` — per-case generation
  metric with `satisfies_minimum()`.

### Episode

- `episode/episode_id.rs` — stable identifier for one realistic synthetic
  episode.
- `episode/episode_theme.rs` — closed theme taxonomy: incident,
  investigation, migration, product decision, memory task and smart writing.
- `episode/episode_objective.rs` — non-empty objective text.
- `episode/capability_target.rs` — capability + minimum examples target.
- `episode/episode_step_plan.rs` — one planned operator step in an episode.
- `episode/synthetic_episode_spec.rs` — aggregate root: episode id + theme +
  objective + non-empty step plan.
- `episode/episode_split_policy.rs` — closed split strategy enum.
- `episode/bounded_ratio.rs` and `episode/stratum_key.rs` — typed parameters
  used by split policies.

### Dataset

- `dataset/synthetic_dataset_blueprint.rs` — `SyntheticDatasetBlueprint`
  with constructors `new` (refuses empty + duplicate case ids) and
  `for_all_capabilities` (one case per `KmpMcpCapability`) plus
  `for_all_generation_targets` (10 KMP tools + `stop` + `escalate`).
- `dataset/synthetic_dataset.rs` — `SyntheticDataset` = dataset_id +
  trajectories.
- `dataset/synthetic_dataset_generation_report.rs` — dataset +
  per-case metrics + `total_generated()` + `every_case_satisfies_minimum()`.

### Corpus quality

Corpus quality is modeled with the same tactical pattern as the shared action
contract:

- one `Specification<CorpusSnapshot>` per rule under `specifications/`;
- `quality/composite_corpus_quality_validator.rs` composes them in stable
  order;
- `quality/corpus_quality_violations.rs` accumulates every violation rather
  than failing fast.

The strict corpus validator registers 13 specs:

- `schema_parse_spec.rs`
- `action_parse_spec.rs`
- `contract_coverage_spec.rs`
- `mode_safety_spec.rs`
- `reference_safety_spec.rs`
- `scope_safety_spec.rs`
- `pagination_safety_spec.rs`
- `write_proof_spec.rs`
- `no_gold_audit_spec.rs`
- `episode_split_spec.rs`
- `duplicate_audit_spec.rs`
- `replay_smoke_spec.rs`
- `frontier_ceiling_spec.rs`

`quality/corpus_snapshot.rs` is the typed subject those specs evaluate.
`quality/corpus_audit_snapshot.rs` carries external audit signals that are
produced by adapters or scripts outside the domain.

### Test support fixtures

`quality/test_support/` contains the v7.2 handcrafted seed corpus used by the
corpus-quality specs. It is intentionally Rust-only and builds every row
through domain constructors; there is no JSON, TOML or deserialization path in
these fixtures.

The five seed episodes are:

- `episode_incident_payments_timeout`
- `episode_software_migration`
- `episode_bug_investigation`
- `episode_product_planning`
- `episode_smart_writing`

`clean_corpus_snapshot()` composes those episodes into a typed
`CorpusSnapshot`. Each corpus-quality spec also has one focused failing
fixture in the same module, for example `corpus_missing_kernel_forward()` or
`corpus_with_write_lacking_read_before_write()`.

When adding a new corpus-quality spec:

1. Add a focused failing fixture to `quality/test_support/snapshots.rs`.
2. Add a clean-corpus test and a failing-fixture test in the spec file.
3. Verify the failing test asserts the specific `ContractViolationCode` and
   a useful `field`.
4. Extend `CompositeCorpusQualityValidator::default_strict()` only after the
   fixture exists.

### Teacher calibration

Teacher calibration lives in the synthetic bounded context because it measures
the policy quality of the teacher that will later generate synthetic operator
trajectories.

Layer placement:

- `operator-synthetic-domain/src/calibration/` defines the typed calibration
  aggregate and value objects: case id, theme, category, subject, accepted
  actions, capability bucket and rationale.
- `operator-synthetic-application/src/ports/` defines
  `CalibrationEpisodeSource` and `TeacherPolicy`.
- `operator-synthetic-application/src/use_cases/evaluate_teacher_calibration_use_case.rs`
  loads cases through the source port, asks the teacher for one action per
  subject, validates the predicted action through the shared strict contract,
  and builds a report.
- `operator-synthetic-infra/src/adapters/jsonl_calibration_episode_source.rs`
  reads runtime JSONL artifacts.
- `operator-synthetic-infra/src/adapters/openai_compatible_teacher_policy.rs`
  calls an OpenAI-compatible chat endpoint.
- `operator-synthetic-cli/src/bin/operator_teacher_calibration.rs` wires the
  adapters and writes `report.json`.

The teacher sees only `CalibrationSubject`: `about`, mode, task family, goal,
allowed tools and visible KMP state. It never receives accepted actions or the
human rationale.

Reports include:

- overall exact-action accuracy;
- tool-selection count;
- strict-contract-valid count;
- shape-failed count;
- per-capability metrics;
- per-category metrics (`happy` vs `adversarial`).

The OpenAI-compatible client is intentionally implemented in
`operator-synthetic-infra` instead of importing evaluation infra. This duplicates
a small HTTP adapter but keeps bounded-context edges clean:

```text
synthetic -> shared
evaluation -> shared
no synthetic <-> evaluation dependency
```

### Errors

- `error/synthetic_domain_error.rs` — `SyntheticDomainError` with
  `EmptyBlueprint`, `DuplicateCase`, episode/split validation errors, plus a
  transparent `Shared` variant for `operator-shared-domain::DomainError`.

## Application map

### Ports

- `ports/synthetic_case_generator.rs` — `SyntheticCaseGenerator` trait:
  takes a `&SyntheticCaseSpec`, returns `Result<Vec<TrainingTrajectory>,
  GenerateSyntheticCaseError>`. Adapters in `operator-synthetic-infra`
  implement this.
- `ports/corpus_source.rs` — `CorpusSource` trait: loads a typed
  `CorpusSnapshot` for quality evaluation. v7.1b defines the port only;
  adapters land later.
- `ports/calibration_episode_source.rs` — reads calibration cases for teacher
  policy evaluation.
- `ports/teacher_policy.rs` — asks a teacher model to choose one operator
  action from a model-facing `CalibrationSubject`.
- `ports/scenario_source.rs` — reads externally authored realistic corpus
  scenarios. The port returns typed `Scenario` values, not raw JSON.
- `ports/scenario.rs` and `ports/scenario_id.rs` — scenario input for
  production corpus generation: id, generation target and model-facing subject.

### Services

- `services/episode_splitter.rs` — applies an `EpisodeSplitPolicy` to a
  slice of `SyntheticEpisodeSpec`.
- `services/episode_split.rs` — result value returned by `EpisodeSplitter`.

### Use cases

- `use_cases/generate_synthetic_dataset_use_case.rs` —
  `GenerateSyntheticDatasetUseCase`. Walks `blueprint.cases()`, delegates
  to the generator port for each, returns a
  `SyntheticDatasetGenerationReport`. Adapter and shared-domain
  errors propagate; per-case minimum failures are surfaced in the report,
  not as hard errors.
- `use_cases/evaluate_corpus_quality_use_case.rs` —
  `EvaluateCorpusQualityUseCase`. Loads a corpus through `CorpusSource`,
  evaluates it through an injected `CorpusQualityValidator`, and returns a
  `CorpusQualityReport`.
- `use_cases/corpus_quality_report.rs` — valid/invalid quality result plus
  accumulated violations.
- `use_cases/evaluate_teacher_calibration_use_case.rs` —
  `EvaluateTeacherCalibrationUseCase`. Loads calibration cases, invokes the
  teacher, compares against multi-accepted gold actions, validates predicted
  actions through the shared strict contract, and returns a
  `TeacherCalibrationReport`.
- `use_cases/teacher_calibration_report.rs` — calibration metrics and gate
  result for v7.2.5.
- `use_cases/build_realistic_corpus_use_case.rs` — v7.3 production corpus
  builder. It reads scenarios, calls the calibrated teacher, validates target
  selection and strict action contract, drops bad rows with explicit
  `DropReason`, and gates the run with `MaxDropRate`.
- `use_cases/realistic_corpus_report.rs` — accepted trajectories, dropped rows,
  drop-rate gate, per-target counts and dropped-by-reason counts.

Teacher calibration subjects may include an optional typed `prepared_action`.
This is the boundary used for prepared write/ingest calibration: the teacher can
see the candidate KMP/MCP action it is being asked to execute, while gold
`accepted_actions` and human rationales remain outside the LLM boundary.

### Errors

- `error/generate_synthetic_case_error.rs` — `GenerateSyntheticCaseError`
  with `Domain` (transparent wrap of `SyntheticDomainError`) and
  `Generator` (adapter-side failure with adapter id + case id +
  message).
- `error/generate_synthetic_dataset_error.rs` —
  `GenerateSyntheticDatasetError` aggregating `Case` and `Domain`.
- `error/corpus_source_error.rs`
- `error/evaluate_corpus_quality_error.rs`
- `error/episode_split_error.rs`
- `error/calibration_episode_source_error.rs`
- `error/teacher_policy_error.rs`
- `error/evaluate_teacher_calibration_error.rs`

## Infra map

- `generators/in_memory_synthetic_case_generator.rs` —
  `InMemorySyntheticCaseGenerator`. Produces one fixed fixture per
  `KmpMcpCapability` and clones it N times to satisfy the spec minimum.
  Used by the end-to-end test and by future contexts that need a stub
  generator (replay smoke tests, training pipeline dry-runs). It rejects
  non-KMP generation targets fail-fast.
- `generators/teacher_backed_synthetic_case_generator.rs` —
  `TeacherBackedSyntheticCaseGenerator`. Builds a model-facing
  `CalibrationSubject`, calls a `TeacherPolicy`, validates that the teacher
  selected the expected generation target and runs the shared strict action
  contract before returning a `TrainingTrajectory`. It covers all
  `SyntheticGenerationTarget` variants: KMP tool calls, `stop` and `escalate`.
- `adapters/jsonl_calibration_episode_source.rs` —
  runtime JSONL adapter for `CalibrationCaseDto`.
- `adapters/jsonl_scenario_source.rs` —
  runtime JSONL adapter for externally authored `ScenarioDto` rows used by
  `operator-realistic-corpus`.
- `adapters/openai_compatible_teacher_policy.rs` —
  OpenAI-compatible teacher adapter for calibration. It performs no JSON
  repair; invalid assistant content is a shape failure.
- `mappers/scenario_mapper.rs` — maps scenario DTOs to typed application
  `Scenario` values.
- `mappers/realistic_corpus_report_mapper.rs` — maps corpus reports and drop
  entries to JSON DTOs for audit artifacts.
- `prompts/teacher_calibration_vN.md` — versioned teacher prompts used by the
  calibration CLI. New prompt versions are evidence artifacts, not automatic
  approval to generate training corpus.

`teacher_calibration_v3.md` introduced the prepared-action rule. v4 kept that
rule and made explicit that goals requiring `beyond_capability` escalation must
not be avoided by speculative memory reads.

## End-to-end test

`crates/operator-synthetic-infra/tests/end_to_end.rs` wires
`SyntheticDatasetBlueprint::for_all_capabilities` →
`GenerateSyntheticDatasetUseCase` → `InMemorySyntheticCaseGenerator` and
asserts:

1. The resulting dataset has `KmpMcpCapability::ALL.len() * minimum`
   trajectories.
2. Every per-case metric satisfies its minimum.
3. Every generated trajectory passes
   `CompositeActionContractValidator::default_strict()` from
   `operator-shared-domain`.

## Pending for later passes

- Scenario libraries for incidents, bug investigations, migrations, product
  decisions, benchmark-like memory tasks and smart writing sessions.
- `scenarios-v1/scenarios.jsonl` production artifact and the script that builds
  it from handcrafted scenario templates.
- Synthetic-context contract DTOs for persisting a
  `SyntheticDatasetGenerationReport` to disk (when a real consumer needs
  it).
