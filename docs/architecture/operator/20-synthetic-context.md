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
operator-synthetic-domain    capabilities, case specs, blueprints, reports
operator-synthetic-application use cases + the SyntheticCaseGenerator port
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

### Dataset

- `dataset/synthetic_dataset_blueprint.rs` — `SyntheticDatasetBlueprint`
  with constructors `new` (refuses empty + duplicate case ids) and
  `for_all_capabilities` (one case per `KmpMcpCapability`).
- `dataset/synthetic_dataset.rs` — `SyntheticDataset` = dataset_id +
  trajectories.
- `dataset/synthetic_dataset_generation_report.rs` — dataset +
  per-case metrics + `total_generated()` + `every_case_satisfies_minimum()`.

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

### Use cases

- `use_cases/generate_synthetic_dataset_use_case.rs` —
  `GenerateSyntheticDatasetUseCase`. Walks `blueprint.cases()`, delegates
  to the generator port for each, returns a
  `SyntheticDatasetGenerationReport`. Adapter and shared-domain
  errors propagate; per-case minimum failures are surfaced in the report,
  not as hard errors.

### Errors

- `error/generate_synthetic_case_error.rs` — `GenerateSyntheticCaseError`
  with `Domain` (transparent wrap of `SyntheticDomainError`) and
  `Generator` (adapter-side failure with adapter id + case id +
  message).
- `error/generate_synthetic_dataset_error.rs` —
  `GenerateSyntheticDatasetError` aggregating `Case` and `Domain`.

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

- Realistic episode domain model for process-level corpus generation.
- Teacher-backed `SyntheticCaseGenerator` adapter (LLM in the loop), with
  strict validation before any teacher output becomes a trajectory.
- Scenario libraries for incidents, bug investigations, migrations, product
  decisions, benchmark-like memory tasks and smart writing sessions.
- Synthetic-context contract DTOs for persisting a
  `SyntheticDatasetGenerationReport` to disk (when a real consumer needs
  it).
