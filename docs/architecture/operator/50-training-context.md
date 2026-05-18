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
                               (this PR)
operator-training-application  use cases + ports (PR B, pending)
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

## Test coverage

28 unit tests in `training-domain` covering: empty/duplicate refusals
in `TaskFamilyDistribution`, provenance distribution-total mismatch,
each `ReadinessGate` variant's evaluation, `ReadinessReport`
construction invariants, `TrainingRun` refusal for unready manifests
and mismatched ids, and round-trip of every value-object parser.

## Pending for later passes

- **PR B — `training-application`**: `AssembleManifest`,
  `EvaluateReadiness`, `BuildTrainingRun` use cases and the ports
  (`TrajectorySource`, `EvaluationReportRepository`, `DatasetWriter`,
  `ManifestWriter`, `TrainerInvoker`).
- **PR C — `training-infra`**: filesystem JSONL dataset writer
  (`{ "prompt": <visible_state_text>, "completion": <action_json> }`
  per ADR 0012 §3), TOML manifest writer, `std::process::Command`
  trainer invoker, CLI integration test against a stub shell script.
- Quality pass after audit.
- Future: DPO / GRPO dataset writers behind the same port; metric
  capture from trainer stdout; manifest schema published under
  `api/training/`.
