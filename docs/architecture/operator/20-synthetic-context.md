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
`KmpMcpCapability` with a minimal canonical fixture; an LLM-teacher
generator and the writer/exec scenario library land in later passes.

The fixture generator is not training-grade. It exists to prove the action
contract, SFT preparation and round-trip pipeline. The realistic training
direction is documented in
[`../../training/operator-realistic-corpus-v7-plan-2026-05-20.md`](../../training/operator-realistic-corpus-v7-plan-2026-05-20.md).

## Crates

```
operator-synthetic-domain    capabilities, episodes, corpus quality specs,
                             case specs, blueprints, reports
operator-synthetic-application use cases, services + generation/corpus ports
operator-synthetic-infra     in-memory adapter; teacher adapter pending
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
  capability + minimum_examples.
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
  `for_all_capabilities` (one case per `KmpMcpCapability`).
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

### Errors

- `error/synthetic_domain_error.rs` — `SyntheticDomainError` with
  `EmptyBlueprint`, `DuplicateCase`, plus a transparent `Shared` variant
  for `operator-shared-domain::DomainError`.

## Application map

### Ports

- `ports/synthetic_case_generator.rs` — `SyntheticCaseGenerator` trait:
  takes a `&SyntheticCaseSpec`, returns `Result<Vec<TrainingTrajectory>,
  GenerateSyntheticCaseError>`. Adapters in `operator-synthetic-infra`
  implement this.
- `ports/corpus_source.rs` — `CorpusSource` trait: loads a typed
  `CorpusSnapshot` for quality evaluation. v7.1b defines the port only;
  adapters land later.

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

## Infra map

- `generators/in_memory_synthetic_case_generator.rs` —
  `InMemorySyntheticCaseGenerator`. Produces one fixed fixture per
  `KmpMcpCapability` and clones it N times to satisfy the spec minimum.
  Used by the end-to-end test and by future contexts that need a stub
  generator (replay smoke tests, training pipeline dry-runs).

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

- Teacher-backed `SyntheticCaseGenerator` adapter (LLM in the loop), with
  strict validation before any teacher output becomes a trajectory.
- Scenario libraries for incidents, bug investigations, migrations, product
  decisions, benchmark-like memory tasks and smart writing sessions.
- Synthetic-context contract DTOs for persisting a
  `SyntheticDatasetGenerationReport` to disk (when a real consumer needs
  it).
