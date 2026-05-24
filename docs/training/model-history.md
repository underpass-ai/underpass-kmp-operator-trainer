# Model history

Short summary of the base models that have been fine-tuned with
LoRA SFT against the KMP/MCP action contract during the pre-disaster
phase of this project. Each row distils dozens of training-run notes
from `rehydration-kernel/docs/product/kernel-tool-operator-model-plan.md`
(authoritative, 772 lines, archival-only). Use this table to avoid
re-litigating model-selection decisions; consult the kernel doc when
you need the full reasoning behind a specific result.

## Current default

**`Qwen/Qwen2.5-0.5B-Instruct`** — the canonical operator base
model. It is the only model that has produced a release candidate
on the strict KMP action contract; every other candidate has been
classified as a useful control or a legacy experiment.

## Models evaluated

| Base model | Params | Status | Best result on strict contract | Notes |
| --- | --- | --- | --- | --- |
| `Qwen/Qwen2.5-0.5B-Instruct` | 0.5B | **Default** | Exact action accuracy 1.0000 on MemoryArena V6 holdout20 (1,124/1,124), zero invalid, zero unbounded | Trains in minutes on a single RTX 3090; tokenizer + LoRA target modules work out of the box; clean generation under the single-JSON contract. |
| `MadeAgents/Hammer2.0-0.5b` | 0.5B | Control | Improved versus its v7 baseline after clean visible-state rerun; ties Qwen on some splits but more verbose | Useful non-Qwen 0.5B-class control. Not a release candidate. |
| `NovachronoAI/LFM2.5-1.2B-Nova-Function-Calling` | 1.2B | Control | Trained cleanly in ~9 min on one RTX 3090 but did not beat the Qwen v7 baseline | Tuned explicitly for function calling. Useful non-Qwen control. Not a release candidate. |
| `meta-llama/Llama-3.2-1B-Instruct` | 1B | Control | Trained cleanly; Apache-2.0 / non-gated; does not beat Qwen | Useful Llama-architecture control. Not a release candidate. |
| `google/functiongemma-270m-it` | 270M | Legacy experiment | Exact action accuracy ≈ 0.5000 with 30 strict prediction failures and zero unbounded calls | Native FunctionGemma tool schemas do not cover the full KMP action surface; treated as legacy / read-only experiment. The `functiongemma_*.py` scripts are intentionally **not** migrated into operator. |
| `falconh1-05b` | 0.5B | Legacy experiment | LongMemEval legacy-v7 result; superseded by newer Qwen baselines | Kept for traceability via `rehydration-kernel/k8s/kernel-operator-falconh1-*` jobs. |

## Contract-learning baseline

The next Operator 0.5B training run must be compared against the last failed
small conformance baseline, not against benchmark-reader results:

| Run | Dataset | Size | Result | Artifact |
| --- | --- | --- | --- | --- |
| Qwen 0.5B LoRA conformance full v4 | legacy KMP/MCP conformance | 58 trajectories | 24.1% exact-action accuracy | `/tmp/kernel-operator-qwen05-conformance-full-v4-policy-eval.json` |

This anchor matters because the current P0 is contract cleanup. If the next
strict run improves sharply, the dataset and wire-contract cleanup mattered. If
it stays near 24.1%, the limitation is probably the 0.5B policy or the task
formulation rather than the legacy dataset shape.

## Current training decision

Do not train a release-candidate model on the contract-v6 fixture corpus from
PR #27.

That corpus is a valid pipeline gate, not a realistic training set. It has
10/10 KMP/MCP tool coverage and strict round-trip validation, but it is built
from canonical cloned fixtures. A high score on it would mostly prove
memorization of the fixture shape.

The next meaningful model-history row should come from the realistic-v7 corpus
plan:

```text
docs/training/operator-realistic-corpus-v7-plan-2026-05-20.md
```

That row must include the v7 frontier ceiling for the same held-out episode
split. A 300-600 row v7 run is a training smoke, not a release-candidate
baseline.

## v8.0 — Qwen2.5-0.5B-Instruct LoRA SFT (2026-05-23)

### Inputs

- Corpus: `../rehydration-kernel-artifacts/operator/realistic-v7-full-v5-1-pr41-20260523T101100Z/trajectories.jsonl`
- Corpus SHA-256: `3b9194f3fc14eb90228662cab3c0a71008b1375e57b0064a2021cd9df416e51e`
- Train JSONL: `/tmp/operator-sft-v8.0/openai_train.jsonl` (1,371 rows, SHA-256 `ca6751f48cd3f9c01ae6b56558b5f99df90dab57dfc1e381bc97a4f3f67eab15`)
- Eval JSONL: `/tmp/operator-sft-v8.0/openai_eval.jsonl` (242 rows, SHA-256 `626eec90c827296c405d75b2395316e3dfe3370ea6fd3d6934906427ec403212`)
- Split: grouped by `about`, eval ratio 0.15
- Base model: `Qwen/Qwen2.5-0.5B-Instruct`

### Hyperparameters

- LoRA: r=16, alpha=32, dropout=0.05
- Target modules: `q_proj,k_proj,v_proj,o_proj,gate_proj,up_proj,down_proj`
- Optimizer schedule: lr=2e-4, cosine scheduler, warmup_ratio=0.03
- Epochs: 3
- Effective batch: 16 (`batch_size=4` per GPU x 4 GPUs x `grad_accum=1`)
- Max length: 2048
- Precision: fp16
- Distributed run: `torchrun --standalone --nproc_per_node=4`
- NCCL: `NCCL_P2P_DISABLE=1`, `NCCL_IB_DISABLE=1`

### Training run

- Kubernetes Job: `underpass-runtime/operator-qwen05-lora-train-4gpu`
- Wall clock: 713.8s (about 12 min)
- Steps: 258 (86 steps/epoch x 3 epochs)
- Final adapter: `/tmp/operator-qwen05-lora-v8.0/adapter_model.safetensors`
- Final adapter SHA-256: `4a5ed6fa2057cb2f20db3289fc51ae114ad32167c4f13db1cfb68a3c8855f7b1`
- Checkpoints: `/tmp/operator-qwen05-lora-v8.0/checkpoint-{86,172,258}/`

Eval metrics:

| Epoch | Step | eval_loss | eval_mean_token_accuracy |
| --- | ---: | ---: | ---: |
| 1 | 86 | 0.0391338058 | 0.9870025739 |
| 2 | 172 | 0.0242082980 | 0.9899247177 |
| 3 | 258 | 0.0235070214 | 0.9900132008 |

### Process deviations

1. Frontier ceiling was not run before training. Mitigation: run it retroactively on the same eval split before interpreting model accuracy.
2. Training observability hardening (TensorBoard + step-level eval) was skipped. The run has per-epoch eval points only.
3. The observer agent was skipped; training was monitored manually through Kubernetes logs.
4. The first 1-GPU Kubernetes Job launch was deleted and replaced by the 4-GPU manifest.
5. Output paths were originally non-versioned. The dataset was renamed to `/tmp/operator-sft-v8.0`; the LoRA directory could not be renamed because it is owned by `nobody` under sticky `/tmp`, so it was copied to `/tmp/operator-qwen05-lora-v8.0` and the original was left intact.
6. Cluster side effects happened in the same session and are not part of the model result: `0.5b.llm.underpassai.com` ingress/TLS was configured, and `underpass-llm-gemma-4-31b-structured` was scaled to zero to free GPUs.
7. The first trained-model predict job used implicit generation defaults (`max_new_tokens=350`, `stop_after_json=false`) and produced 19 `incomplete_json` failures. The run was preserved at `/tmp/operator-qwen05-predictions-v8.0/`; the repo predict manifest was updated to use explicit `--max-new-tokens 2048 --temperature 0.0 --stop-after-json`, then rerun successfully.

### Artifacts

- Dataset: `/tmp/operator-sft-v8.0/`
- Adapter: `/tmp/operator-qwen05-lora-v8.0/`
- Frontier ceiling predictions: `../rehydration-kernel-artifacts/operator/frontier-ceiling-v8.0-20260523T140341Z/predictions.jsonl`
- Frontier ceiling policy eval: `../rehydration-kernel-artifacts/operator/frontier-ceiling-v8.0-20260523T140341Z/policy_eval_report.txt`
- First trained prediction attempt, incomplete: `/tmp/operator-qwen05-predictions-v8.0/` (223 predictions, 19 `incomplete_json` failures)
- Trained predictions, final: `/tmp/operator-qwen05-predictions-v8.0-2048-jsonstop/predictions.jsonl`
- Trained prediction summary, final: `/tmp/operator-qwen05-predictions-v8.0-2048-jsonstop/summary.json`
- Trained policy eval, final: `/tmp/operator-qwen05-predictions-v8.0-2048-jsonstop.policy_eval_report.txt`

### Results

Both frontier and trained runs were evaluated without constrained decoding. The
frontier run used `gpt-4o-mini` at temperature 0.0 and produced 12 shape-invalid
rows; PR #43 evaluator changes count those rows as evaluated failures instead
of aborting the report. The trained run used free JSON generation with
`--stop-after-json` and a 2048-token completion budget.

Global metrics:

| Metric | Frontier `gpt-4o-mini` | Qwen 0.5B LoRA | Delta |
| --- | ---: | ---: | ---: |
| total | 242 | 242 | - |
| parsed | 230 | 242 | +12 |
| shape_invalid | 12 (4.96%) | 0 (0.00%) | -4.96 pp |
| exact_match | 90 (37.19%) | 180 (74.38%) | +37.19 pp |
| tool_match | 230 (95.04%) | 242 (100.00%) | +4.96 pp |
| contract_valid | 220 (90.91%) | 242 (100.00%) | +9.09 pp |

Per-capability exact match:

| Capability | Frontier | Qwen 0.5B LoRA | Delta |
| --- | ---: | ---: | ---: |
| `<stop/escalate>` | 6/6 (100.00%) | 6/6 (100.00%) | +0.00 pp |
| `kernel_ingest` | 0/19 (0.00%) | 0/19 (0.00%) | +0.00 pp |
| `kernel_wake` | 58/58 (100.00%) | 58/58 (100.00%) | +0.00 pp |
| `kernel_ask` | 9/30 (30.00%) | 13/30 (43.33%) | +13.33 pp |
| `kernel_near` | 0/18 (0.00%) | 15/18 (83.33%) | +83.33 pp |
| `kernel_goto` | 3/3 (100.00%) | 3/3 (100.00%) | +0.00 pp |
| `kernel_rewind` | 0/13 (0.00%) | 13/13 (100.00%) | +100.00 pp |
| `kernel_forward` | 0/35 (0.00%) | 33/35 (94.29%) | +94.29 pp |
| `kernel_trace` | 0/23 (0.00%) | 23/23 (100.00%) | +100.00 pp |
| `kernel_inspect` | 14/15 (93.33%) | 15/15 (100.00%) | +6.67 pp |
| `kernel_write_memory` | 0/22 (0.00%) | 1/22 (4.55%) | +4.55 pp |

The frontier `kernel_ingest` denominator includes the 12 shape-invalid rows
allocated back to their ground-truth capability; the printed CLI per-tool bucket
only shows parsed rows.

### Decision

v8.0 closes as the first interpretable trained baseline. The 0.5B specialist
beats the frontier ceiling by +37.19 percentage points on exact action, reaches
100% tool selection and 100% contract validity, and confirms the research
hypothesis that a small fine-tuned model can beat a frontier model on bounded
operator action structure.

The closure is not a production serving claim. Write payload exactness remains
weak (`kernel_ingest` 0/19, `kernel_write_memory` 1/22), and constrained
decoding is still backlog for v8.1. Do not fallback to 1.5B until the v8.1
work determines whether the write gap is caused by model capacity, evaluation
normalization, or the lack of deterministic prepared-payload resolution during
prediction.

## v8.1 — action_correctness metric (2026-05-24)

v8.1 adds a second scoring layer next to the legacy exact-match metric. The
legacy metric remains unchanged for historical comparison. The new
`action_correctness` metric evaluates each action field under an explicit mode:

- `Exact`: byte-equivalent display value is required.
- `SchemaValid`: the value may differ from the gold value, but it must satisfy
  the field's current domain shape.
- `Permissive`: free-text content is accepted when it is non-empty and typed
  correctly.

The field policy lives in `operator-shared-domain`, not in the evaluator. This
keeps scoring rules attached to the action/tool contract instead of hiding them
in reporting code.

One important implementation detail: v8.1 follows the current domain contract.
Generated IDs such as `idempotency_key`, `correlation_id` and `causation_id`
are currently typed as non-empty strings in the action domain, so their
`SchemaValid` rule is non-empty string. If production needs UUID-only values,
that is a separate contract change: add typed UUID/generated-ID value objects,
then tighten the correctness rule.

### v8.0 artifacts rescored

No model was retrained and no new paid calls were made. The reports reuse the
existing v8.0 predictions:

- Frontier report:
  `../rehydration-kernel-artifacts/operator/frontier-ceiling-v8.0-20260523T140341Z/action_correctness_report.txt`
- Trained report:
  `/tmp/operator-qwen05-action_correctness_report.txt`

The trained report was written outside the prediction directory because
`/tmp/operator-qwen05-predictions-v8.0-2048-jsonstop/` is owned by `nobody` and
is not writable from the local shell.

### Global comparison

| Metric | Frontier `gpt-4o-mini` | Qwen 0.5B LoRA | Delta |
| --- | ---: | ---: | ---: |
| legacy exact_match | 90/242 (37.19%) | 180/242 (74.38%) | +37.19 pp |
| action_correctness | 111/242 (45.87%) | 197/242 (81.40%) | +35.53 pp |
| tool_selection | 230/242 (95.04%) | 242/242 (100.00%) | +4.96 pp |
| shape_invalid | 12/242 (4.96%) | 0/242 (0.00%) | -4.96 pp |

The new metric raises both models because it stops treating generated/free-text
fields as byte-exact requirements. It does not hide structural failures: invalid
shapes still count as incorrect, and structural arrays such as memory entries,
relations and evidence remain exact.

### Trained model: top failing fields

| Field | Correct | Failure pattern |
| --- | ---: | --- |
| `memory.entries[*]` | 0/19 (0.00%) | `kernel_ingest` structural payload mismatch |
| `memory.relations[*]` | 0/19 (0.00%) | `kernel_ingest` structural payload mismatch |
| `related[*]` | 1/22 (4.55%) | `kernel_write_memory` related refs mismatch |
| `memory.evidence[*]` | 6/19 (31.58%) | `kernel_ingest` evidence mismatch |
| `memory.dimensions[*]` | 10/19 (52.63%) | `kernel_ingest` dimension mismatch |
| `anchor` | 15/18 (83.33%) | `kernel_near` anchor mismatch |
| `window` | 46/48 (95.83%) | `kernel_forward` window mismatch |

Generated fields such as `idempotency_key`, `provenance.source_agent`,
`provenance.observed_at`, `provenance.correlation_id` and
`provenance.causation_id` were all correct under the current `SchemaValid`
rules for the trained model.

### Trained model: per-tool action correctness

| Capability | action_correctness | Status |
| --- | ---: | --- |
| `<stop/escalate>` | 6/6 (100.00%) | clean |
| `kernel_wake` | 58/58 (100.00%) | clean |
| `kernel_ask` | 30/30 (100.00%) | clean under permissive query text |
| `kernel_goto` | 3/3 (100.00%) | clean |
| `kernel_rewind` | 13/13 (100.00%) | clean |
| `kernel_trace` | 23/23 (100.00%) | clean |
| `kernel_inspect` | 15/15 (100.00%) | clean |
| `kernel_forward` | 33/35 (94.29%) | just below 95% read-side gate |
| `kernel_near` | 15/18 (83.33%) | below 95% read-side gate |
| `kernel_write_memory` | 1/22 (4.55%) | below 99% write-side gate |
| `kernel_ingest` | 0/19 (0.00%) | below 99% write-side gate |

### v8.1 conclusion

`action_correctness` confirms the v8.0 finding while making the remaining work
more precise. The 0.5B model is strong on bounded read-side action structure and
fully contract-valid on this eval split, but write payload reconstruction is not
production-ready. The next useful training iteration is not a blind 1.5B
fallback; it is targeted v8.1.2 data work for the failing structural fields:
`memory.entries[*]`, `memory.relations[*]`, `memory.evidence[*]`,
`memory.dimensions[*]` and `related[*]`.

## Why Qwen 0.5B and not something larger?

The kernel's design goal (see [`feedback_small_models`] in the
project memory) is: **make a small specialist model capable on
bounded graph tasks**, not improve frontier models. Qwen 0.5B is
small enough that:

- A single 24 GB consumer GPU can fine-tune it in minutes.
- Inference latency stays well below the KMP per-call budget.
- The model carries no general-purpose capability the operator
  would have to suppress under the strict contract.

Larger models (LFM 1.2B, Llama 1B) reach the same exact-match score
on holdout but burn more compute and tokens for no gain in the
bounded-task setting.

## Adding a new candidate

1. Confirm the base model meets the [licensing requirements] for
   your downstream use (Apache-2.0 / non-gated preferred; gated
   models need an HF token mount in the K8s job).
2. Copy `k8s/qwen05-lora-train.yaml` to `k8s/<model>-lora-train.yaml`
   and override `MODEL_ID` + LoRA target modules if needed.
3. Train; run `predict_operator_sft.py`; feed the predictions to
   `ValidateTrainedRunUseCase`.
4. Add a row above with the result. Keep the table to **one line per
   model**; archive the full reasoning in a per-run note under
   `docs/training/runs/<date>-<model>.md` when results justify the
   detail.

[`feedback_small_models`]: ../../README.md
[licensing requirements]: ../../LICENSE
