# Training Bounded Context

The `training` bounded context **prepares a training run**: it
assembles a dataset, captures its provenance, evaluates readiness
gates against an `EvaluationReport`, packages everything into a
manifest, and produces a `TrainingRun` value that represents a run
that is **allowed to launch**.

It does **not** run a model. Running is delegated to an external
trainer wired through a CLI adapter in `training-infra` (PR C). See
[ADR 0012](decisions/0012-training-context-design.md).

## Crates

```
operator-training-domain       value objects, readiness gates,
                               TrainingManifest, TrainingRun aggregate
operator-training-application  use cases + ports (this PR)
operator-training-infra        JSONL dataset writer, TOML manifest
                               writer, CLI trainer invoker (PR C,
                               pending)
```

Allowed dependencies (per `01-bounded-contexts.md`):

```
training-domain      ──▶  shared-domain
training-application ──▶  shared-domain, evaluation-domain,
                          training-domain
training-infra       ──▶  shared-domain, shared-infra,
                          training-domain, training-application
```

No dependency on `synthetic`, `replay`, `runtime`,
`benchmark-adapters` or any `rehydration-*` crate. Cross-context
data (`TrainingTrajectory`, `EvaluationReport`) flows through the
shared and evaluation domain types respectively.

## Domain map

### Identifiers

- `ids/mod.rs` — re-exports `TrainingRunId` from `shared-domain`. The
  re-export keeps consumer imports inside the training crate boundary
  even though the identifier itself is shared vocabulary.

### Provenance

- `provenance/dataset_source.rs` — `DatasetSource`: opaque label
  (synthetic run id, git SHA, path) describing where the dataset came
  from. The training context does not interpret the label; it
  preserves it for traceability.
- `provenance/content_hash.rs` — `ContentHash`: prefixed hash string
  (e.g., `sha256:...`). The algorithm prefix is part of the value;
  verification is the consumer's job.
- `provenance/task_family_distribution.rs` /
  `provenance/task_family_distribution_entry.rs` — distribution of
  trajectories across task families. The collection refuses to be
  built with duplicate families. An empty distribution is allowed
  (caller has not yet computed per-family counts).
- `provenance/dataset_provenance.rs` — `DatasetProvenance`: aggregates
  source + content_hash + trajectory_count + distribution. The
  constructor refuses to build a value whose non-empty distribution
  total disagrees with the declared `trajectory_count`.

### Readiness

- `readiness/pass_rate_percent.rs` — `PassRatePercent`: pass-rate
  threshold in `[0.0, 1.0]`. Refuses NaN, ±∞ and out-of-range values.
- `readiness/readiness_gate.rs` — `ReadinessGate` closed enum:
  `MinimumTrajectories(PositiveCount)`,
  `MinimumTaskFamilyCoverage(PositiveCount)`,
  `MinimumEvaluationPassRate(PassRatePercent)`. `evaluate(provenance,
  evaluation)` produces a `ReadinessCheck`. Adding a gate forces every
  exhaustive `match` to update; wildcards are forbidden by convention.
- `readiness/readiness_outcome.rs` — `ReadinessOutcome` per-check
  verdict (`Passed` | `Failed { reason: NonEmptyString }`).
- `readiness/readiness_check.rs` — `ReadinessCheck`: pairs the gate
  with its outcome.
- `readiness/readiness_report.rs` — `ReadinessReport`: aggregates
  checks. Refuses to be built from an empty gate list (zero gates is a
  configuration smell). `is_ready()` is true iff every check passed.

### Trainer target

- `trainer/trainer_command.rs` — `TrainerCommand`: the executable to
  invoke (e.g., `sft-trainer`).
- `trainer/base_model_id.rs` — `BaseModelId`: opaque base-model label
  (e.g., `Qwen/Qwen2.5-1.5B-Instruct`).
- `trainer/output_directory.rs` — `OutputDirectory`: where the trainer
  writes artefacts. Held as a non-empty string; filesystem resolution
  lives in `training-infra`.
- `trainer/trainer_target.rs` — `TrainerTarget`: composes the three
  above.

### Manifest

- `manifest/training_manifest.rs` — `TrainingManifest`: holds
  `(run_id, dataset_provenance, trainer_target, readiness_report)`.
  Representable in any readiness state — failing manifests are
  legitimate values (used for diagnostics or archival). It is the
  aggregate root, **not** the manifest, that refuses unready inputs.

### Aggregate root

- `run/training_run.rs` — `TrainingRun`. Named constructor
  `TrainingRun::new(id, manifest)` refuses to build when:
  - The manifest's `run_id` differs from the run's `id`
    (`ManifestRunIdMismatch`).
  - The manifest's readiness report fails any gate
    (`NotReady { failed, total, summary }`). The summary is the
    concatenation of every failed gate's reason.

  A `TrainingRun` value therefore represents a run that **is allowed
  to launch**.

### Errors

- `errors/training_domain_error.rs` — `TrainingDomainError` (thiserror)
  with `Shared(DomainError)` transparent wrapper plus training-specific
  variants (`EmptyReadinessReport`, `NotReady`,
  `ManifestRunIdMismatch`, `DuplicateTaskFamilyInDistribution`,
  `DistributionTotalMismatch`).
- `errors/training_result.rs` — `TrainingResult<T>` alias.

## Application map

### Ports (`training-application/src/ports/`)

- `trajectory_source.rs` — `TrajectorySource` trait. `fetch_all() ->
  Vec<TrainingTrajectory>`. Adapters in `training-infra` cover
  filesystem, network, synthetic-handoff, in-memory test fixtures.
- `dataset_writer.rs` — `DatasetWriter` trait. `write(trajectories) ->
  DatasetWriteOutcome`. Writes whatever format the adapter targets
  (JSONL SFT first; DPO / raw / parquet land as additional adapters).
- `dataset_write_outcome.rs` — `DatasetWriteOutcome`: `content_hash` +
  `trajectory_count` + `distribution`. The use case turns this into a
  `DatasetProvenance`; the writer never sees the provenance type.
- `manifest_writer.rs` — `ManifestWriter` trait. `write(&manifest) ->
  ()`. TOML adapter ships in PR C.
- `trainer_invoker.rs` — `TrainerInvoker` trait. `invoke(&target) ->
  TrainerInvocationOutcome`. Adapters wrap external trainer binaries;
  no in-process trainer.
- `trainer_invocation_outcome.rs` — `TrainerInvocationOutcome` closed
  enum: `Success { exit_code }` | `Failed { exit_code: Option<i32> }`
  (signal-killed processes have `None`).

### Errors (`training-application/src/errors/`)

Each port has its own adapter-agnostic error type
(`TrajectorySourceError`, `DatasetWriteError`, `ManifestWriteError`,
`TrainerInvokerError`). The use cases flatten them into
`TrainingApplicationError`, which also wraps `TrainingDomainError`
transparently via `#[from]`. `TrainingApplicationResult<T>` is the
alias every use case returns.

### Use cases (`training-application/src/use_cases/`)

- `evaluate_readiness_use_case.rs` — pure: runs every `ReadinessGate`
  against `DatasetProvenance` + `EvaluationReport`, collects the
  `ReadinessCheck`s into a `ReadinessReport`. No ports.
- `assemble_manifest_use_case.rs` — pure: composes a
  `TrainingManifest` from `(run_id, provenance, trainer_target,
  readiness)`. No ports.
- `build_training_run_request.rs` — input DTO for the build use case.
  Groups `(run_id, trainer_target, dataset_source, gates, evaluation)`
  so callers do not thread five positional arguments through.
- `build_training_run_use_case.rs` — top-level orchestration. Reads
  trajectories from the `TrajectorySource`, writes them via the
  `DatasetWriter`, builds `DatasetProvenance` from the writer's
  `DatasetWriteOutcome`, evaluates readiness, assembles the manifest,
  **writes the manifest first** (so an unready manifest is on disk
  for diagnostics), then constructs the `TrainingRun`. Returns either
  the ready run or surfaces `TrainingDomainError::NotReady` /
  `ManifestRunIdMismatch` from `TrainingRun::new`.
- `launch_training_run_use_case.rs` — wraps the `TrainerInvoker`
  call. The run aggregate root guarantees readiness, so there is
  nothing to check domain-side; the use case just forwards the
  manifest's `TrainerTarget` to the invoker and returns its outcome.

## Test coverage

- **`training-domain`**: 28 unit tests covering empty/duplicate
  refusals in `TaskFamilyDistribution`, provenance distribution-total
  mismatch, every `ReadinessGate` variant's evaluation,
  `ReadinessReport` construction invariants, `TrainingRun` refusal
  for unready manifests and mismatched ids, and round-trip of every
  value-object parser.
- **`training-application`**: 10 unit tests using inline trait test
  doubles (stub trajectory source, stub dataset writer, recording &
  failing manifest writers, recording & failing trainer invokers).
  Covers happy path of `BuildTrainingRunUseCase`, every port-error
  propagation path, the "unready manifest written before run refused"
  contract, and `LaunchTrainingRunUseCase` happy + failure paths.

## Pending for later passes

- **PR C — `training-infra`**: filesystem JSONL dataset writer
  (`{ "prompt": <visible_state_text>, "completion": <action_json> }`
  per ADR 0012 §3), TOML manifest writer, `std::process::Command`
  trainer invoker, CLI integration test against a stub shell script.
- Quality pass after audit.
- Future: DPO / GRPO dataset writers behind the same port; metric
  capture from trainer stdout; manifest schema published under
  `api/training/`.
