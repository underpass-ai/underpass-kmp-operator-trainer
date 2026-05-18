# Operator training runbook

End-to-end recipe for training a small operator policy (Qwen 0.5B +
LoRA SFT) and validating it before declaring a release candidate.
This runbook assumes the holdout dataset has already been produced
upstream (synthetic generation or external benchmark adapter — the
synthetic side will gain a `operator-synthetic-cli` in a later PR;
benchmark adapters are kernel-side and stay there per ADR 0001).

The pipeline has five stages: **prepare → train → predict → score →
gate**. The first three are Python (industry-standard SFT/LoRA tools
have no Rust equivalent at parity); the last two are Rust use cases
in `operator-evaluation-application` and `operator-training-application`.

```
+--------------------------+    +--------------------+
| trajectory-v1 JSONL      |--->| prepare            | (Python)
| (synthetic or benchmark) |    | _sft_dataset.py    |
+--------------------------+    +---------+----------+
                                          |
                                          v
                                +---------+----------+
                                | openai_train.jsonl |
                                | openai_eval.jsonl  |
                                +---------+----------+
                                          |
              +---------------------------+
              v
+-------------+--------------+   +-------------------+
| train_operator_sft_lora.py |-->| LoRA adapter dir  |
| (k8s/qwen05-lora-train)    |   +---------+---------+
+----------------------------+             |
                                           v
                              +------------+--------+
                              | predict_operator_   |
                              | sft.py              |
                              | (k8s/predict)       |
                              +------------+--------+
                                           |
                                           v
                              +------------+--------+
                              | predictions.jsonl   |
                              | summary.json        |
                              +------------+--------+
                                           |
                                           v
                              +------------+--------+
                              | ValidateTrainedRun  | (Rust)
                              | UseCase             |
                              +------------+--------+
                                           |
                                           v
                              +------------+--------+
                              | EvaluationReport    |
                              | + is_passing(rate)  |
                              +---------------------+
```

## 0. Prerequisites

- A Kubernetes cluster with at least one GPU node and the NVIDIA
  device plugin installed (see [`k8s/README.md`](../../k8s/README.md)
  for the assumptions baked into the templates).
- This repo cloned at `/home/tirso/ai/developents/operator` (or the
  `hostPath` in the K8s jobs adjusted).
- Python 3.11+ available on a workstation if you want to run the
  scripts directly without Kubernetes.
- A trajectory-v1 JSONL on the host filesystem. For now, produce it
  by:
  - copying one from the kernel's existing exports (e.g.,
    `rehydration-kernel/scripts/operator/` historical artefacts), or
  - running operator's synthetic generation library directly (no
    CLI yet — see the audit punch list).

## 1. Prepare the SFT dataset

The Python `prepare_operator_sft_dataset.py` script turns a
trajectory-v1 JSONL into the conversational SFT shape the trainer
consumes:

```bash
python scripts/operator/prepare_operator_sft_dataset.py \
    --input /tmp/trajectories.jsonl \
    --output-dir /tmp/operator-sft
```

That produces `/tmp/operator-sft/openai_{train,eval}.jsonl` with the
`{"messages": [system, user, assistant]}` shape every row should
have. The script validates each row against the strict KMP action
contract and **fails fast** on shape errors — fix the upstream
trajectory exporter rather than the script.

## 2. Train

Apply the K8s job and wait for completion (typically 20-40 minutes
for a few-hundred-row dataset on a single RTX 3090 / A100):

```bash
kubectl apply -f k8s/qwen05-lora-train.yaml
kubectl wait --for=condition=complete --timeout=2h \
    job/operator-qwen05-lora-train
kubectl logs job/operator-qwen05-lora-train | tail -50
```

The LoRA adapter lands at `/tmp/operator-qwen05-lora` on the host
node. Verify with `ls /tmp/operator-qwen05-lora`.

Workstation alternative (single GPU, no Kubernetes):

```bash
python scripts/operator/train_operator_sft_lora.py \
    --train-jsonl /tmp/operator-sft/openai_train.jsonl \
    --eval-jsonl  /tmp/operator-sft/openai_eval.jsonl \
    --model-id Qwen/Qwen2.5-0.5B-Instruct \
    --output-dir /tmp/operator-qwen05-lora \
    --epochs 3 --batch-size 2 --grad-accum 8 \
    --max-length 2048 --fp16
```

## 3. Predict against the holdout

```bash
kubectl apply -f k8s/qwen05-lora-predict.yaml
kubectl wait --for=condition=complete --timeout=1h \
    job/operator-qwen05-lora-predict
```

The predictor writes `/tmp/operator-qwen05-predictions/{predictions,summary,failures}.{jsonl,json,jsonl}`
on the host.

Workstation alternative:

```bash
python scripts/operator/predict_operator_sft.py \
    --dataset-jsonl /tmp/operator-sft/openai_eval.jsonl \
    --model-id Qwen/Qwen2.5-0.5B-Instruct \
    --adapter /tmp/operator-qwen05-lora \
    --output  /tmp/operator-qwen05-predictions \
    --batch-size 8 --force
```

## 4. Score and gate (Rust validation loop)

Today there is **no `operator-evaluation-cli` binary yet** (tracked
in the post-disaster gap analysis). The Rust library has the full
flow — wire it via a small adapter binary or call it from a test:

```rust
use operator_evaluation_infra::adapters::jsonl_predictions_reader::JsonlPredictionsReader;
use operator_training_application::ports::{
    predictor::Predictor, predictor_target::PredictorTarget,
};
use operator_training_application::use_cases::{
    validate_trained_run_request::ValidateTrainedRunRequest,
    validate_trained_run_use_case::ValidateTrainedRunUseCase,
};
use operator_training_domain::readiness::pass_rate_percent::PassRatePercent;
use operator_training_infra::adapters::{
    composite_policy_evaluator::CompositePolicyEvaluator,
    jsonl_predictions_reader_adapter::JsonlPredictionsReaderAdapter,
    process_predictor_invoker::ProcessPredictorInvoker,
};

// 1. Wire the three ports to filesystem adapters.
let predictor = ProcessPredictorInvoker::new();
let reader = JsonlPredictionsReaderAdapter::new(
    "/tmp/operator-qwen05-predictions/predictions.jsonl",
);
let evaluator = CompositePolicyEvaluator::new(/* your ActionContractValidator */);

let use_case = ValidateTrainedRunUseCase::new(predictor, reader, evaluator);

// 2. Build the request with the holdout's ground-truth trajectories
//    (the same trajectories you fed `prepare_operator_sft_dataset.py`).
let request = ValidateTrainedRunRequest::new(
    PredictorTarget::new(
        /* TrainerCommand for the predictor binary or script */,
        /* BaseModelId */,
        /* adapter_directory */,
        /* dataset_path */,
        /* output_directory */,
    )?,
    ground_truth_trajectories,
);

// 3. Execute and read the verdict.
let outcome = use_case.execute(&request)?;
let passed = outcome.is_passing(PassRatePercent::parse(0.90)?);
println!("predictions: {}", outcome.predictor_outcome().predictions());
println!("exact_match_rate: {:.4}",
         outcome.evaluation_report().exact_match_rate());
println!("verdict: {}", if passed { "PASS" } else { "FAIL" });
```

Until the CLI lands, integrate this snippet into a small custom
binary or a `#[test]` for ad-hoc runs. Both the
`is_passing(...)` projection and the underlying counts live on
`ValidateTrainedRunOutcome` so callers can log + gate from a single
return value.

## 5. Promote or iterate

- **Pass**: archive the dataset hash, the LoRA adapter weights, and
  the `summary.json` + evaluation report next to the
  `TrainingManifest` TOML the build phase wrote (the
  `TomlManifestWriter` records dataset provenance + readiness
  gates). Add a one-line entry to
  [`model-history.md`](model-history.md) when the result is
  publication-grade.
- **Fail**: inspect `predictions.jsonl` and
  `compare_operator_policy_details.py` (in
  `scripts/operator/`) for per-row failure modes. Common causes:
  contract violations (use `audit_operator_sft_no_gold.py` on the
  dataset), wrong `kernel_*` tool selection (almost always a
  prompt-template drift), or hyperparameter regressions (start with
  the kernel's `*-v5` defaults and bisect).

## Known gaps

This runbook calls out two CLI gaps explicitly because they require
manual workarounds today:

1. **No `operator-synthetic-cli`** — the synthetic context can
   generate trajectories programmatically (see
   `operator-synthetic-application`), but there is no command-line
   binary yet. Stage trajectories from the kernel's existing exports
   or write a small custom binary against the library API.
2. **No `operator-evaluation-cli`** — the validation step in §4
   above currently requires a custom binary or a test harness; a
   `policy-eval` / `contract-coverage` / `llm-baseline` CLI trio
   will follow in dedicated PRs.

When those CLIs land, the runbook collapses to four `kubectl apply`
+ one `cargo run` invocation.
