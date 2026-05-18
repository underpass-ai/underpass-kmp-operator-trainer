# Evaluation Bounded Context

The `evaluation` bounded context **scores predictions against ground
truths**. It does not generate trajectories, replay them, or train
anything: it consumes pairs of (ground-truth `TrainingTrajectory`,
`PredictedAction`) and emits a report.

The first pass keeps the surface in-memory: the application use case
receives a slice of `EvaluationPair` values and returns a
`EvaluationReport`. Adapters that stream predictions from JSONL files
or push reports to dashboards belong in a later pass.

## Crates

```
operator-evaluation-domain      prediction pair, outcome, report, per-tool metric
operator-evaluation-application EvaluateOperatorPolicyUseCase
```

No `operator-evaluation-contract` and no `operator-evaluation-infra`
crate today. See
[decisions/0009-evaluation-context-skips-contract-and-infra.md](decisions/0009-evaluation-context-skips-contract-and-infra.md).

## Domain map

### Prediction

- `prediction/predicted_action.rs` — `PredictedAction` = trajectory id +
  `OperatorAction`. The trajectory id couples a prediction to its ground
  truth without dragging the whole trajectory in.
- `prediction/evaluation_pair.rs` — `EvaluationPair` = ground-truth
  `TrainingTrajectory` + `PredictedAction`. The named constructor
  refuses to build a pair whose trajectory ids disagree.

### Outcome

- `outcome/prediction_evaluation_outcome.rs` —
  `PredictionEvaluationOutcome`. Three derived flags:
  - `is_contract_valid`: the prediction passes
    `ActionContractValidator` against the ground truth's mode +
    visible state.
  - `is_exact_match`: `prediction == ground_truth.target_action`.
  - `is_tool_match`: same high-level choice. For two `ToolCall` actions,
    that means same `KernelTool`. For two `Stop` actions or two
    `Escalate` actions, true regardless of payload. Otherwise false.

### Report

- `report/tool_evaluation_metric.rs` — `ToolEvaluationMetric`, the
  per-bucket counters. `tool == None` is the bucket for outcomes whose
  ground truth was `Stop` or `Escalate`.
- `report/evaluation_report.rs` — `EvaluationReport`. Carries every
  outcome plus per-tool aggregates and overall rates (exact, tool,
  contract).

### Errors

- `error/evaluation_domain_error.rs` — `EvaluationDomainError` with
  `TrajectoryIdMismatch` and a transparent `Shared` variant for
  `operator-shared-domain::DomainError`.

## Application map

### Use cases

- `use_cases/evaluate_operator_policy_use_case.rs` —
  `EvaluateOperatorPolicyUseCase`. Generic over `V:
  ActionContractValidator`. The execute method loops over pairs, asks
  the validator for contract violations using the ground-truth context,
  builds one outcome per pair and returns a `EvaluationReport`.

### Errors

- `error/evaluate_operator_policy_error.rs` —
  `EvaluateOperatorPolicyError`. Wraps `EvaluationDomainError` for
  forward compatibility.

## Pending for later passes

- Adapter port + JSONL adapter to load predictions emitted by a model
  runner.
- Adapter port + writer to persist `EvaluationReport` as a JSON
  document for dashboards.
- Per-`OperatorMode` aggregation (read vs. write vs. writer-pre-read).
- Confidence-weighted metrics once predictions carry logits / log-probs.
