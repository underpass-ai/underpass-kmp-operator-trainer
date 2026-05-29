# Training Bounded Context

> **Divergence note (2026-05-29):** the dataset-writer serialization is
> structurally correct, but the v8.x corpora emit **un-anonymized literal domain
> refs** in model-facing `visible_state`, diverging from the anonymization
> requirement (kernel plan:182-186). Anonymization is now mandatory in
> `prepare_operator_sft_dataset.py`. See
> [`../../training/DIVERGENCE_AND_CORRECTIVE_PLAN_2026-05-29.md`](../../training/DIVERGENCE_AND_CORRECTIVE_PLAN_2026-05-29.md).

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
operator-training-application  use cases + ports
operator-training-infra        JSONL dataset writer, TOML manifest
                               writer, std::process::Command trainer
                               invoker (this PR)
```

Allowed dependencies (per `01-bounded-contexts.md`):

```
training-domain      ──▶  shared-domain
training-application ──▶  shared-domain, evaluation-domain,
                          training-domain
training-infra       ──▶  shared-domain, shared-infra,
                          training-domain, training-application,
                          evaluation-domain, evaluation-application,
                          evaluation-infra
```

The cross-context edge `training-infra → evaluation-application` is
intentional and tightly scoped: `CompositePolicyEvaluator` (an
adapter in `training-infra`) wraps `EvaluateOperatorPolicyUseCase`
(in `evaluation-application`) so the use-case-facing
`PolicyEvaluator` port stays inside `training-application` and the
application layer never imports `evaluation-application` directly.
Similarly, `JsonlPredictionsReaderAdapter` wraps
`evaluation-infra::JsonlPredictionsReader` to keep
`training-application` decoupled from `evaluation-infra`. These
adapters are the only place in the operator where one bounded
context's infra touches another bounded context's application or
infra layer; treat any new edge of this shape as a smell unless it
follows the same wrapping pattern.

No dependency on `synthetic`, `replay`, `runtime`, or any
`rehydration-*` crate. Cross-context data (`TrainingTrajectory`,
`EvaluationReport`, `StepKeyedPrediction`) flows through the shared
and evaluation domain types respectively. Benchmark adapters are not
an Operator concern; they live in the kernel (`rehydration-kernel`).

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
- `validate_trained_run_use_case.rs` — post-train validation
  orchestrator. Invokes the `Predictor` over a holdout dataset,
  reads `predictions.jsonl` through a `PredictionsReader` adapter,
  joins step-keyed predictions to the in-memory ground-truth
  trajectories by `step_id`, and feeds the resulting
  `EvaluationPair`s to a `PolicyEvaluator`. Returns a
  `ValidateTrainedRunOutcome` pairing the predictor's process
  outcome with the model-level `EvaluationReport`. Predictions for
  steps not in the ground-truth set are dropped silently (same
  shape as the kernel evaluator).
- `validate_trained_run_request.rs` / `validate_trained_run_outcome.rs`
  — input DTO and return value object for the use case.

### Additional ports (`training-application/src/ports/`)

- `predictor.rs` + `predictor_target.rs` + `predictor_outcome.rs` —
  `Predictor` trait, the domain-typed `PredictorTarget` (command,
  base model, adapter dir, dataset path, output dir) and the
  `PredictorOutcome` returned by the adapter (paths + summary
  counts).
- `predictions_reader.rs` — `PredictionsReader` trait. Reads
  `Vec<StepKeyedPrediction>` from whatever source the adapter
  fronts.
- `policy_evaluator.rs` — `PolicyEvaluator` trait. Wraps the
  evaluation-application use case so `training-application` never
  imports `operator-evaluation-application` directly.

### Additional errors (`training-application/src/errors/`)

`PredictorError` (`SpawnFailure`, `NonZeroExit`),
`PredictionsReadError` (`SourceUnavailable`, `InvalidRow`,
`ShapeViolation`) and `PolicyEvaluatorError` (`DomainFailure`).
All three flatten into `TrainingApplicationError` via `#[from]`.

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

## Infra map

### Adapters (`training-infra/src/adapters/`)

- `jsonl_sft_dataset_writer.rs` — `JsonlSftDatasetWriter`. Filesystem
  `DatasetWriter` that emits one JSONL line per `TrainingTrajectory`,
  shaped `{"prompt": <json>, "completion": <json>}` per ADR 0012 §3.
  `prompt` is `serde_json::to_string(&VisibleStateDto)` (a stable,
  lossless JSON serialisation of the visible state — a richer
  natural-language renderer can land behind a future `PromptRenderer`
  port). `completion` is `serde_json::to_string(&OperatorActionDto)`.
  Bytes are SHA-256-hashed in flight and the per-task-family
  distribution is built during the write pass, so the returned
  `DatasetWriteOutcome` is self-consistent. Output is always
  newline-terminated; the writer overwrites any pre-existing target.
- `toml_manifest_writer.rs` — `TomlManifestWriter`. Filesystem
  `ManifestWriter` that serialises a `TrainingManifest` via a typed
  TOML DTO tree (see `src/dto/manifest_*.rs`). Each readiness gate is
  rendered as `[[readiness.gate]]` with its `kind` (closed string),
  human-readable `target`, `status` and (when failed) `reason`. The
  top-level `[readiness] overall` is `"ready"` iff every gate passed.
- `process_trainer_invoker.rs` — `ProcessTrainerInvoker`. Wraps
  `std::process::Command`. Builds the trainer command line as
  `<command> --base-model <base_model> --output-dir
  <output_directory>`, waits synchronously, and maps the exit status
  to `TrainerInvocationOutcome::Success { exit_code: 0 }`,
  `Failed { exit_code: Some(code) }` or `Failed { exit_code: None }`
  (signal-killed processes). stdout / stderr are inherited; this
  adapter never parses trainer logs.
- `process_predictor_invoker.rs` — `ProcessPredictorInvoker`.
  Sibling of the trainer invoker for the Python
  `predict_operator_sft.py` script in `scripts/operator/`. Builds
  `<command> --model-id <base_model> --adapter <adapter_dir>
  --dataset-jsonl <dataset_path> --output <output_dir>`, redirects
  stdin to `/dev/null`, waits, and reads `<output_dir>/summary.json`
  to populate `PredictorOutcome.predictions` and `failures`.
- `jsonl_predictions_reader_adapter.rs` — `PredictionsReader`
  adapter that wraps the `operator-evaluation-infra` JSONL reader
  and translates its evaluation-infra error type into the
  training-application `PredictionsReadError`.
- `composite_policy_evaluator.rs` — `PolicyEvaluator` adapter that
  wraps `EvaluateOperatorPolicyUseCase` from
  `operator-evaluation-application`. Keeps the training-application
  layer decoupled from evaluation-application; only the infra crate
  knows about both.

### DTOs (`training-infra/src/dto/`)

Per ADR 0004 the domain has no `serde` dependency, so every wire
shape lives in this crate as a `Serialize`-derived DTO mapped from
the domain at the adapter boundary:

- `jsonl_sft_example_dto.rs` — the per-line `{prompt, completion}` DTO.
- `manifest_dto.rs` + `manifest_run_dto.rs` +
  `manifest_dataset_dto.rs` + `manifest_readiness_dto.rs` +
  `manifest_readiness_gate_dto.rs` +
  `manifest_trainer_target_dto.rs` — the manifest TOML shape. Each
  file declares exactly one public type, respecting `1 file = 1
  class` without an exception entry.

### Test coverage

- **`tests/jsonl_dataset_writer.rs`** — 8 filesystem round-trip tests:
  one line per trajectory plus expected DTO fields, hash determinism
  across two writers with the same input, write-failure mapping for
  unwritable paths, refusal to write zero trajectories (the
  `PositiveCount` invariant), trailing newline contract, `Send +
  Sync` smoke, idempotent overwrite, pre-existing-target overwrite.
- **`tests/toml_manifest_writer.rs`** — 3 round-trip tests asserting
  the parsed TOML tree (not the byte layout, which can drift with the
  `toml` crate): every section present with expected values,
  `overall = "ready"` iff all gates pass, write-failure mapping for
  unwritable paths.
- **`tests/process_trainer_invoker.rs`** — 4 integration tests
  (`#[cfg(unix)]`) against stub shell scripts: exit 0 →
  `Success{exit_code:0}`, exit 7 → `Failed{exit_code:Some(7)}`,
  unknown command → `SpawnFailure`, and a captured-argv assertion
  proving `--base-model` and `--output-dir` reach the trainer.
- **`tests/end_to_end.rs`** — 2 end-to-end tests wiring
  `BuildTrainingRunUseCase` with the real filesystem adapters: happy
  path writes both files and returns a ready `TrainingRun`; the
  unready-readiness path proves the manifest still lands on disk
  with `overall = "not_ready"` even when `TrainingRun::new` refuses
  the run.

## Pending for later passes

- Quality pass after audit.
- Future: DPO / GRPO dataset writers behind the same port; metric
  capture from trainer stdout; manifest schema published under
  `api/training/`; richer `PromptRenderer` port if the JSON prompt
  shape proves too verbose for the operator policy LLM.
