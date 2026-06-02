# `k8s/` — Kubernetes job templates for Operator training and prediction

This directory holds the minimal K8s `Job` templates that drive the
Python SFT/LoRA pipeline at [`scripts/operator/`](../scripts/operator/).
Both templates use `hostPath` volumes for the repo and `/tmp` so the
job can run on a single-node cluster (k3s, kind, docker-desktop) with
a local GPU. For multi-node or remote-cluster runs, replace the
`hostPath` volumes with the storage class your cluster supports
(persistent volume claims, CSI volumes, etc.).

## What's here

- `qwen05-lora-train.yaml` — fine-tunes `Qwen/Qwen2.5-0.5B-Instruct`
  with LoRA via `train_operator_sft_lora.py`. Writes the adapter
  weights to `${OUTPUT_DIR}`.
- `qwen05-lora-predict.yaml` — runs the trained adapter over a
  holdout JSONL via `predict_operator_sft.py`. Writes
  `predictions.jsonl` + `summary.json` + `failures.jsonl` to
  `${OUTPUT_DIR}`.

Both jobs read overridable env vars with defaults that match the
runbook walkthrough. The **train** job reads `DATASET_DIR`,
`OUTPUT_DIR`, `MODEL_ID`; the **predict** job reads `DATASET_JSONL`,
`ADAPTER_DIR`, `OUTPUT_DIR`, `MODEL_ID`, `MAX_NEW_TOKENS`.

What's intentionally **not** here:

- The dataset-specific variants the kernel keeps in
  `rehydration-kernel/k8s/kernel-operator-qwen05-*` (45 files,
  per-benchmark and per-cut). Those are kernel/benchmark concerns;
  operator stays generic.
- Production Helm charts or ArgoCD `Application` manifests. Add them
  in a follow-up once these templates stabilise.

## Prerequisites

- Kubernetes cluster with GPU node(s) and the NVIDIA device plugin
  installed. The **train** template requests `nvidia.com/gpu: 4` and
  runs `torchrun --nproc_per_node=4` (DDP); the **predict** template
  requests `nvidia.com/gpu: 1`.
- The repo cloned at `/home/tirso/ai/developents/operator` (or edit
  the `volumes.repo.hostPath` in both templates to point at your
  checkout).
- `/tmp` writable on the node hosting the pod.
- A holdout SFT JSONL on the host filesystem (see the runbook for how
  to produce one via `scripts/operator/prepare_operator_sft_dataset.py`
  or a future `operator-synthetic-cli`).

## Running

End-to-end example, assuming a holdout already at
`/tmp/operator-sft/openai_{train,eval}.jsonl`:

```bash
# 1. Train. Suspends until completion (typically 20-40 min on the
#    4-GPU DDP config for a few-hundred-trajectory dataset).
kubectl apply -f k8s/qwen05-lora-train.yaml
kubectl wait --for=condition=complete --timeout=2h \
  job/operator-qwen05-lora-train
kubectl logs job/operator-qwen05-lora-train | tail -50

# 2. The LoRA adapter is now at /tmp/operator-qwen05-lora on the
#    host. Verify:
ls /tmp/operator-qwen05-lora

# 3. Predict against the eval split.
kubectl apply -f k8s/qwen05-lora-predict.yaml
kubectl wait --for=condition=complete --timeout=1h \
  job/operator-qwen05-lora-predict
kubectl logs job/operator-qwen05-lora-predict | tail -50

# 4. Predictions are now at /tmp/operator-qwen05-predictions/.
#    Hand off to operator's Rust validation loop (see runbook).
cat /tmp/operator-qwen05-predictions/summary.json
```

## Tuning

The defaults aim at a small-dataset baseline. Common knobs:

- **Train epochs / batch size**: edit `args.*` in
  `qwen05-lora-train.yaml`. Manifest args: `epochs=3`, `batch_size=4`,
  `grad_accum=1`, `max_length=2048`, `bf16` (across 4 DDP ranks →
  effective batch 16).
- **Base model**: set `MODEL_ID` to any HuggingFace model whose
  tokenizer + LoRA target modules are compatible. The Python script
  takes `--lora-target-modules` if you need to deviate from the
  Qwen-friendly default
  (`q_proj,k_proj,v_proj,o_proj,gate_proj,up_proj,down_proj`).
- **GPU count / multi-GPU**: the train template already runs 4-GPU
  DDP (`torchrun --nproc_per_node=4`, `--device-map none`, `gpu: "4"`).
  To scale down to a single GPU, set `--nproc_per_node=1` and lower
  `resources.{limits,requests}.nvidia.com/gpu` to `1`; to scale up,
  raise both together. See the kernel's `*-4gpu-*-job.yaml` variants
  for additional multi-GPU baselines.
- **Skipping training**: pass `--validate-only` to the trainer for a
  schema-only dry-run that does not require a GPU.

## Hand-off to operator's Rust validation loop

The predict job's output is exactly the shape
`operator-evaluation-infra::JsonlPredictionsReader` consumes. The
Rust use case `ValidateTrainedRunUseCase` (in
`operator-training-application`) then joins those predictions with the
ground-truth `TrainingTrajectory` set and feeds them to
`EvaluateOperatorPolicyUseCase`. The end-to-end recipe (with the
relevant CLI bindings — once they exist — or with a direct library
call) is in [`../docs/training/runbook.md`](../docs/training/runbook.md).
