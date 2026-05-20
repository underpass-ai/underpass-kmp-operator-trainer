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
- A trajectory-v1 JSONL on the host filesystem. The fastest way to
  produce one is via `operator-synthesize` (see §0a). Alternative
  sources: a kernel-side export, or a custom binary against
  `operator-synthetic-application`.

## 0a. Synthesize a trajectory JSONL

`operator-synthesize` wires the fixture-grade
`InMemorySyntheticCaseGenerator` against the canonical
`for_all_capabilities` blueprint and writes the result as
`TrainingTrajectoryDto` JSONL — the exact shape every downstream
tool in this runbook reads:

```bash
cargo run --release -p operator-synthetic-cli --bin operator-synthesize -- \
    --dataset-id dataset:smoke-$(date +%Y-%m-%d) \
    --minimum-examples 4 \
    --output /tmp/trajectories.jsonl
```

The output is deterministic given the same `--dataset-id` and
`--minimum-examples`. The CLI prints a per-case coverage report so
you know exactly how many trajectories each `KmpMcpCapability`
contributed. **Note**: the in-memory generator is fixture-grade —
one canonical trajectory per capability, cloned N times. It satisfies
the strict KMP action contract but does not reflect realistic
operator behaviour; a teacher-model-backed generator will land in a
later pass.

## 0b. Audit a trajectory JSONL with `operator-contract-coverage`

Before feeding a trajectory JSONL into the rest of the pipeline,
verify it covers every `KernelTool` variant and that every row
validates against the strict KMP action contract:

```bash
cargo run --release -p operator-evaluation-cli --bin operator-contract-coverage -- \
    --trajectories /tmp/trajectories.jsonl \
    --require-full-coverage \
    --require-zero-invalid
```

The binary prints per-tool / per-mode counts plus the tool-coverage
ratio, then gates on the two flags: `--require-full-coverage` fails
if any `KernelTool` variant has zero trajectories;
`--require-zero-invalid` fails if any row's action violates the
strict contract. Omitting both flags prints metrics and exits 0.

Run this in CI against any dataset destined for training to catch
exporter regressions (missing tools, drifted action shapes) before
they reach the trainer.

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

## 4b. Or score offline with `operator-policy-eval`

Once the predictor has written `predictions.jsonl`, you can score it
against any ground-truth JSONL without re-running the predictor:

```bash
cargo run --release -p operator-evaluation-cli --bin operator-policy-eval -- \
    --predictions  /tmp/operator-qwen05-predictions/predictions.jsonl \
    --ground-truth /tmp/operator-sft/eval.jsonl \
    --min-pass-rate 0.9
```

The binary joins predictions and ground truth by `step_id`, runs
`EvaluateOperatorPolicyUseCase` under the strict KMP contract, prints
per-tool / per-mode metrics, and exits 0 or 1 based on the
`--min-pass-rate` threshold (omit the flag to print metrics only).
Useful for inspecting a historical run, comparing against a different
contract version, or gating a publication candidate.

## 4c. Or replay predictions against a live KMP with `operator-replay`

If you have a running kernel MCP JSON-RPC endpoint, you can execute
each predicted action against the real KMP and measure tool-call
success rate / failure modes:

```bash
cargo run --release -p operator-replay-cli --bin operator-replay -- \
    --predictions  /tmp/operator-qwen05-predictions/predictions.jsonl \
    --ground-truth /tmp/operator-sft/eval.jsonl \
    --mcp-endpoint http://localhost:8080 \
    --min-success-rate 0.95
```

The binary joins predictions and ground truth by `step_id` to resolve
the trajectory id each outcome attaches to, dispatches each predicted
action through `HttpKmpMcpClient`, and prints the resulting
`ReplayReport` (total, successful tool calls, failed tool calls,
`stop/escalate` count, tool-call success rate). With
`--min-success-rate` the exit code is 0 PASS / 1 FAIL against the
threshold; omit it to print metrics only.

Use this when you want to know whether the trained policy survives
contact with the real kernel — not just whether it predicts the
same bytes the SFT label has.

## 4d. Or call a frontier LLM with `operator-llm-baseline`

To measure the ceiling for your dataset, drive the same SFT JSONL
through a frontier LLM via its OpenAI-compatible chat-completions
endpoint and reuse `operator-policy-eval` on the result:

```bash
cargo run --release -p operator-evaluation-cli --bin operator-llm-baseline -- \
    --sft        /tmp/operator-sft/eval.jsonl \
    --output     /tmp/operator-baseline-gpt4o \
    --api-base   https://api.openai.com/v1 \
    --api-key-file /tmp/openai.txt \
    --model      gpt-4o-mini \
    --max-failures 0
```

For vLLM behind the kernel ingress, point `--api-base` at
`https://llm.underpassai.com/v1`; for Anthropic, at
`https://api.anthropic.com/v1` — every backend the pipeline targets
speaks the same OpenAI chat-completions contract. The CLI writes
`predictions.jsonl` (same shape as `predict_operator_sft.py`),
`failures.jsonl` (per-row HTTP / parse failures with the raw
response preserved), and `summary.json`. Pipe the predictions back
through `operator-policy-eval` (§4b) to compare the fine-tuned
0.5B against the frontier model under the same strict KMP contract.

This run hits a paid endpoint per row — use `--limit N` for smoke
checks before turning it loose on a full holdout.

## Known gaps

This runbook calls out the remaining CLI gaps because they require
manual workarounds today:

1. **`operator-synthesize` is fixture-grade** — the canonical
   `InMemorySyntheticCaseGenerator` produces one trajectory per
   `KmpMcpCapability`, cloned N times. The trajectories satisfy the
   strict KMP action contract but do not reflect realistic operator
   behaviour. A teacher-model-backed generator is on the roadmap.
2. **CLI parity with the pre-disaster kernel is complete.**
   `operator-policy-eval` covers prediction scoring (§4b),
   `operator-replay` covers live KMP execution (§4c),
   `operator-contract-coverage` covers dataset audits (§0b), and
   `operator-llm-baseline` covers frontier-LLM ceilings (§4d). The
   only outstanding work is the teacher-model-backed synthetic
   generator listed above.
