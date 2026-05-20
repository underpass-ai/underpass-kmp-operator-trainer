# `scripts/operator/` — KMP operator trainer (Python)

This directory holds the Python pipeline that fine-tunes a small
Qwen 0.5B operator policy with LoRA over Operator-shaped SFT data.

It is intentionally **Python**, not Rust: the SFT + LoRA ecosystem
(`transformers` + `peft` + `trl` + `accelerate`) is the industry
standard and has no Rust equivalent at parity for training. The Rust
side of the operator repo (`crates/operator-training-*`) handles
orchestration, manifests, readiness gates, and the post-train
validation loop; it invokes these scripts as opaque subprocesses
through the `ProcessTrainerInvoker` and `ProcessPredictorInvoker`
adapters in `operator-training-infra`.

These scripts were migrated wholesale from
`rehydration-kernel/scripts/operator/` after the in-kernel operator
attempt was scrapped (see `docs/architecture/operator/decisions/0001-independent-repo.md`).
The FunctionGemma-native scripts in the original directory were not
migrated — they are legacy experiments that fail fast on the strict
KMP action contract and are out of scope for the new operator
training surface.

## Pipeline

```text
                 ┌─────────────────────────┐
                 │ kernel-operator-        │
                 │ trajectory-v1 JSONL     │  (produced by kernel
                 │ (one trajectory / line) │   trajectory-export
                 └────────────┬────────────┘   binaries or by
                              │                operator's
                              │                synthetic context
                              ▼
            prepare_operator_sft_dataset.py
                              │
                              │  emits {"messages":[system,user,assistant]}
                              ▼
                 ┌─────────────────────────┐
                 │ openai_train.jsonl      │
                 │ openai_eval.jsonl       │  (conversational SFT)
                 └────────────┬────────────┘
                              │
                              ▼
              train_operator_sft_lora.py
                              │
                              │  Qwen2.5-0.5B-Instruct + LoRA (peft + trl)
                              ▼
                 ┌─────────────────────────┐
                 │ LoRA adapter directory  │
                 │ (output_dir/adapter*)   │
                 └────────────┬────────────┘
                              │
                              ▼
              predict_operator_sft.py
                              │
                              │  emits predictions.jsonl
                              ▼
                 ┌─────────────────────────┐
                 │ predictions.jsonl       │  (one prediction / line)
                 └────────────┬────────────┘
                              │
                              ▼
                 (operator-evaluation-infra,  ─┐
                  PR D in Rust workspace)       │  Score against
                                                │  ground truth →
                                                │  EvaluationReport
                                                │  → readiness
                                                ▼
                              ┌─────────────────────────┐
                              │ Post-train pass / fail  │
                              └─────────────────────────┘
```

## Scripts

- **`prepare_operator_sft_dataset.py`** — turns
  `kernel-operator-trajectory-v1` JSONL into a conversational SFT
  dataset `{"messages":[system, user, assistant]}` per row.
  Validates each row against the strict KMP action contract
  (`kernel-operator-action-contract-v1`) before emitting it. Refuses
  duplicate `id`s and duplicate user/assistant content (model-row
  collisions).
- **`train_operator_sft_lora.py`** — loads the train + eval JSONL,
  fine-tunes `Qwen/Qwen2.5-0.5B-Instruct` with LoRA via TRL's
  `SFTTrainer`. Writes the adapter weights to `--output-dir`.
  Accepts `--validate-only` for a no-GPU lint pass.
- **`predict_operator_sft.py`** — loads the LoRA adapter, runs
  inference over the eval JSONL, validates each predicted action
  against the strict action contract, and writes
  `predictions.jsonl` plus a `summary.json` describing exact-match
  counts and contract validity. The output of this script is what
  `operator-evaluation-infra` reads to build an `EvaluationReport`.
- **`compare_operator_policy_details.py`** — detailed diff between
  predictions and ground truth, useful for human review when a run
  fails the readiness gates.
- **`audit_operator_sft_no_gold.py`** — audits an SFT dataset to
  ensure it never leaks benchmark gold answers into the user
  message. Run this on any new dataset before training.
- **`deanonymize_operator_predictions.py`** — for benchmark
  comparison only: maps anonymized refs in `predictions.jsonl` back
  to the original benchmark ids. Not used in the main training loop.

## Wire formats

Operator pins its formats to what these scripts read and write:

- **Input to `prepare_operator_sft_dataset.py`**:
  `kernel-operator-trajectory-v1` JSONL. One trajectory per line
  with `step_id`, `visible_state`, `target_action`, etc. (see kernel
  `scripts/operator/README.md` for the historical spec).
- **Output of `prepare_operator_sft_dataset.py`** /
  **input to `train_operator_sft_lora.py`** and
  `predict_operator_sft.py`:
  `{"id":..., "step_id":..., "messages":[{"role":"system","content":...}, {"role":"user","content":...}, {"role":"assistant","content":...}]}`
  one row per line.
- **Output of `predict_operator_sft.py`**: `predictions.jsonl`,
  one prediction per line, fields documented inline in the script.
- **Summary**: `summary.json` next to `predictions.jsonl` —
  exact match count, contract validity count, totals. The Rust
  validation loop reads this for its readiness check.

## Running

The k8s jobs in `rehydration-kernel/k8s/kernel-operator-qwen05-*` are
the reference invocations. From this repo, the same pipeline runs
locally as:

```bash
# 1. Prepare (run on whatever produces trajectory-v1 JSONL)
python scripts/operator/prepare_operator_sft_dataset.py \
    --trajectories trajectories.jsonl \
    --output /tmp/operator-sft

# 2. Train
python scripts/operator/train_operator_sft_lora.py \
    --train-jsonl /tmp/operator-sft/openai_train.jsonl \
    --eval-jsonl  /tmp/operator-sft/openai_eval.jsonl \
    --model-id Qwen/Qwen2.5-0.5B-Instruct \
    --output-dir /tmp/operator-lora-out \
    --epochs 3 --batch-size 2 --grad-accum 8 \
    --max-length 2048 --fp16

# 3. Predict
python scripts/operator/predict_operator_sft.py \
    --dataset-jsonl /tmp/operator-sft/eval.jsonl \
    --model-id Qwen/Qwen2.5-0.5B-Instruct \
    --adapter /tmp/operator-lora-out \
    --output  /tmp/operator-predictions

# 4. Validate (Rust side, lands in PR D)
# operator-training-application::ValidateTrainedRunUseCase
# reads /tmp/operator-predictions and builds an EvaluationReport.
```

## Python dependencies

See `requirements.txt` in this directory. Pinned to the versions
proven by the kernel's k8s training jobs.

## Tests

Run the local contract smoke before spending GPU time:

```bash
bash scripts/operator/round_trip_smoke.sh
```

The smoke synthesizes a tiny `TrainingTrajectoryDto` dataset, audits
10/10 KMP tool coverage, prepares 5 SFT rows, validates train/predict
inputs without loading a model, writes stub predictions from the
ground-truth actions, and scores them with `operator-policy-eval`.

To run the same gate against an already generated corpus/SFT pair, pass the
prepared SFT directory. If `OPERATOR_SMOKE_TRAJECTORIES` is omitted, the smoke
uses `${OPERATOR_SMOKE_SFT_DIR}/all_trajectories.jsonl`:

```bash
OPERATOR_SMOKE_SFT_DIR=/path/to/sft \
  bash scripts/operator/round_trip_smoke.sh
```

## Contract v6 corpus

PR #27 introduced a reproducible builder for the P0.4/P0.5 contract corpus:

```bash
bash scripts/operator/build_contract_v6_corpus.sh
```

By default it writes outside the git repo under
`../rehydration-kernel-artifacts/operator/<run-id>/` so generated JSONL does
not pollute source control. The builder:

- synthesizes a new `TrainingTrajectoryDto` JSONL from the current typed
  Operator contract;
- requires full `operator-contract-coverage` over the source, train and eval
  trajectory files;
- prepares SFT with one explicit eval group per KMP/MCP tool, so both train and
  eval keep 10/10 tool coverage;
- audits model-facing rows for gold leakage;
- validates train and predict inputs without loading a model;
- runs `round_trip_smoke.sh` against the generated SFT, not the tiny fixture.
