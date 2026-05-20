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
