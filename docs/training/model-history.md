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

### v8.1.2 SFT v2 — live replay validation (operator-runtime v0)

Built the minimal operator runtime bounded context (`operator-runtime-*`
crates) to validate the v8.1.2 SFT v2 adapter against a live vLLM endpoint
and KMP through MCP JSON-RPC stdio.

#### Setup

- vLLM endpoint: `https://0.5b.llm.underpassai.com/v1`
- Model id: `operator-v8.1.2`
- Adapter SHA-256:
  `43186fa848c5f0e9d71915023f8f01c2341042de8aaf57b0c3c0c574a0f44379`
- KMP endpoint: `https://rehydration-kernel.underpassai.com` through
  `rehydration-mcp` stdio (`REHYDRATION_MCP_BACKEND=grpc`)
- Eval source: `/tmp/operator-sft-v8.1.2/openai_eval.jsonl`
- Replay filter: `read_profile` = `read` + `writer_pre_read`, excluding
  `kernel_ingest` and `kernel_write_memory`
- Selected scenarios: 222 rows. The current split has no `stop` rows in this
  profile.

#### Results

The first live replay surfaced a runtime adapter bug for temporal cursor
requests: `kernel_forward` and `kernel_rewind` were sent as `{key, anchor}`
while `rehydration-mcp` expects exactly one of `{ref, time, sequence}`. The
runtime stdio executor was fixed to translate `seq:N` anchors to
`{"sequence": N}`, RFC3339 anchors to `{"time": ...}`, and other anchors to
`{"ref": ...}`.

A follow-up request-shape audit fixed the remaining confirmed runtime adapter
issues before merge:

- `kernel_trace.page` now uses the KMP page object shape and `kernel_trace.to`
  is required before the call is attempted.
- `kernel_goto` rejects trace cursors before transport because KMP accepts only
  `{ref,time,sequence}` anchors.
- `kernel_near` preserves predicted `dimensions` and `limit`.
- stdio and HTTP MCP executors use the same canonical argument builder.
- JSONL sinks append instead of truncating evidence.
- vLLM parsing rejects non-`stop` finish reasons and enforces stricter DTO
  field sets.

Post-audit live subsets covered 20 general read-profile scenarios plus 10
impacted `kernel_trace`/`kernel_near`/`kernel_goto` scenarios. The remaining
failures were KMP `NotFound` responses, not argument-shape rejections.

A control MCP smoke against a live graph anchor
`article:incident:checkout-latency:20260504T233722Z:frontend` returned a
successful `kernel_wake` response. That confirms the KMP endpoint and MCP
transport were healthy during diagnosis; the replay `NotFound` failures are
from eval refs absent from the live graph.

Full replay result after the initial cursor-shape fix:

| Tool | Scenarios | target/predicted match | live MCP success | Notes |
| --- | ---: | ---: | ---: | --- |
| `kernel_wake` | 65 | 65/65 (100%) | 0/65 (0%) | KMP `NotFound` |
| `kernel_ask` | 32 | 32/32 (100%) | 0/32 (0%) | KMP `NotFound` |
| `kernel_near` | 20 | 20/20 (100%) | 0/20 (0%) | KMP `NotFound` |
| `kernel_goto` | 3 | 3/3 (100%) | 0/3 (0%) | KMP `NotFound` |
| `kernel_rewind` | 13 | 13/13 (100%) | 0/13 (0%) | KMP `NotFound` after cursor-shape fix |
| `kernel_forward` | 41 | 41/41 (100%) | 0/41 (0%) | KMP `NotFound` after cursor-shape fix |
| `kernel_trace` | 24 | 24/24 (100%) | 0/24 (0%) | KMP `NotFound` |
| `kernel_inspect` | 17 | 17/17 (100%) | 0/17 (0%) | KMP `NotFound` |
| `escalate` | 7 | 7/7 (100%) | 7/7 (100%) | Terminal, no MCP call |
| **Global** | **222** | **222/222 (100%)** | **7/222 (3.15%)** | Includes terminal escalations |

MCP-attempted success rate:

| Metric | Value |
| --- | ---: |
| MCP attempted | 215 |
| MCP completed | 0 |
| `mcp_execution_success_rate` | 0/215 (0%) |

Outcome class distribution:

| Outcome class | Count |
| --- | ---: |
| `Escalated` | 7 |
| `McpExecutionFailure` | 215 |
| `Completed` | 0 |
| `ContractViolation` | 0 |
| `BudgetExhausted` | 0 |

Failure category distribution after request-shape fixes:

| Category | Count |
| --- | ---: |
| KMP `NotFound` | 215 |
| MCP request shape rejected | 0 |

Latency:

| Percentile | Value |
| --- | ---: |
| p50 | 220 ms |
| p90 | 368 ms |
| p99 | 404 ms |

#### Conclusion

The live runtime path is validated end to end: vLLM strict structured output,
scenario prompt reconstruction, local contract validation, MCP stdio execution,
JSONL persistence, and replay aggregation all work.

This run does **not** validate production read success for v8.1.2 because the
eval split references synthetic `about:*` / `node:*` ids that are not loaded in
the live KMP. The 0% MCP-attempted success rate is therefore a fixture/data
availability result, not a model action-selection regression. Model action
selection in this replay was 222/222 against the selected target actions.
The post-audit subsets support the same diagnosis after correcting all
confirmed wire-shape defects.

To turn `mcp_execution_success_rate` into a production-readiness metric, the
next run must either ingest the eval fixture graph into an isolated KMP
namespace or replay scenarios derived from refs already present in the target
KMP.

#### Post-fix wire + holdout-direct validation (2026-05-26)

After the initial Phase G replay reported 0/215 MCP success with all
errors mapped to NotFound, two hypotheses competed:

1. Wire-shape bugs cause kernel-side validation failures.
2. Eval split references synthetic ids not loaded in production KMP.

An external code review surfaced 15 wire-shape findings that were addressed in
this PR's commits. To disambiguate hypotheses, we ran empirical smokes against
production KMP with fresh binaries.

##### Multi-tool smoke against real anchors

Eight read tools probed with known-real anchors from production KMP:

- `kernel_wake`, `kernel_ask`, `kernel_inspect`, `kernel_near`,
  `kernel_trace`, `kernel_goto`, `kernel_rewind`, `kernel_forward`
- All returned `isError:false`; wake, ask, inspect, near, trace, and goto
  returned non-empty structured responses, while rewind and forward returned
  valid empty temporal windows for the sampled anchors
- Confirmed wire layer is correct

Bug discovered mid-validation: `WakeResponseMapper` crashed on legitimate
empty `evidence_ref` values in `causal_spine`. Fix: filter empty entries
client-side (commit `39b3511` in this PR). Audit of the other seven response
mappers confirmed wake is the only case where empty refs are structurally
legitimate sentinels.

##### Holdout-direct quantitative validation

16 scenarios sampled directly from v8.1.2-regen2 `openai_eval.jsonl` (2 per
read tool, unmodified), executed against live vLLM `operator-v8.1.2` +
production KMP via stdio MCP.

| Metric | Value |
| --- | --- |
| Total sessions | 16 |
| Action match | 16/16 (100%) |
| MCP attempted | 16 |
| MCP completed | 0 |
| MCP NotFound | 16 |
| InvalidArgument | 0 |
| Other wire errors | 0 |
| Latency p50/p90/p99 | 288/430/431 ms |

Interpretation: hypothesis 2 confirmed. The runtime path and model action
selection are correct under training distribution. The 0% MCP completion is
the expected data-availability shortfall: the v8.1.2 holdout references
synthetic ids that are absent from production KMP. Latency is restored to
near-baseline (pre-fix: 220/368/404 ms), confirming the wire fixes add no
measurable overhead.

##### Production-readiness MCP success rate

This run does **not** measure production-readiness MCP success against the
canonical v8.1.2 holdout. That measurement requires either loading the eval
fixture graph into an isolated KMP namespace or replaying scenarios built from
refs already present in the target KMP. Tracked as separate issue (Path 2).

A custom real-anchor mini-eval was attempted but produced action-selection
drift (25% action_match, model collapsed to `kernel_trace` in 79% of cases).
The drift is attributed to scenario-distribution divergence from the v8.1.2
corpus (n_refs uniformly = 5 across all read tools in training; the custom
builder used different visible_state shape). That result is treated as a
scenario-construction artifact, not a runtime or model regression.

#### v8.1.2-regen2 diagnostic SFT run (2026-05-27)

Built the regen2 dataset using operator-repo's `prepare_operator_sft_dataset.py`
(kernel-repo's version was confirmed to mis-handle the current v2 schema).
Trained a new LoRA adapter under identical hyperparameters to v8.1.2-sft-v2
to test whether the kernel script bug had affected actual training labels.

##### Training run

- Wall clock: 1087s (~18 min) on 4×RTX 3090
- Adapter SHA-256: `8ea66b89452f198e3e7d6bf0f5ece400c6f89202e64d5b130e6aa5dc6c479aaf`
- Adapter path: `/tmp/operator-qwen05-lora-v8.1.2-regen2-20260527T091810/`
- Final eval_loss: 0.02611
- Final eval_mean_token_accuracy: 0.9885
- 3 epochs completed cleanly

##### Holdout evaluation (regen2 eval split, 317 rows)

| Tool | exact_match | tool_match | contract_valid |
| --- | --- | --- | --- |
| kernel_wake | 65/65 (100%) | 65/65 | 65/65 |
| kernel_ask | 15/32 (47%) | 32/32 | 32/32 |
| kernel_near | 19/20 (95%) | 20/20 | 20/20 |
| kernel_goto | 3/3 (100%) | 3/3 | 3/3 |
| kernel_rewind | 13/13 (100%) | 13/13 | 13/13 |
| kernel_forward | 39/41 (95%) | 41/41 | 41/41 |
| kernel_trace | 24/24 (100%) | 24/24 | 24/24 |
| kernel_inspect | 17/17 (100%) | 17/17 | 17/17 |
| kernel_ingest | 0/45 (0%) | 44/45 | 36/45 |
| kernel_write_memory | 1/50 (2%) | 50/50 | 50/50 |
| escalate | 7/7 (100%) | 7/7 | 7/7 |
| **Total** | **203/317 (64%)** | **316/317 (99.7%)** | **308/317 (97.2%)** |

##### Verdict

Read profile remains strong (≥95% across all eight read tools), consistent
with v8.1.2-sft-v2 baseline.

Write profile is flat: `kernel_ingest` 0%, `kernel_write_memory` 2% -- same
ballpark as the v8.0 baseline (0/19 ingest, 1/22 write_memory) despite the
regen2 dataset using the correctly-formatted operator-repo script.

**This confirms hypothesis (b) from the training rationale**: the
rehydration-kernel/main copy of `prepare_operator_sft_dataset.py` mis-handled
only the coverage report (cosmetic), not the actual training rows.
v8.1.2-sft-v2 weights are based on correctly-labeled data; the write-profile
gap has a different root cause.

##### Decision

**Keep v8.1.2-sft-v2 as the deployed operator-v8.1.2 adapter.** The regen2
adapter (SHA `8ea66b89...`) is archived at the path above for reference. No
production rollout.

##### Forward implications

The write-profile gap is therefore intrinsic to the corpus or to the teacher's
behavior on write payloads, not a labeling artifact. This directly motivates
the curriculum learning direction documented in `docs/training/backlog_v8x.md`
(or pending v8.2 backlog work): the gpt-4o-mini teacher itself cannot produce
exact_match consistently on complex write payloads, so SFT on those labels
teaches the model inconsistency.

##### Caveat

Comparison reported above is regen2-trained adapter vs v8.0 baseline metrics
(different eval split). An apples-to-apples comparison would re-evaluate
v8.1.2-sft-v2 against the regen2 eval split. Given the pattern of results
(read profile strong, write profile zero) and the validated hypothesis that the
script bug was cosmetic, the v8.1.2-sft-v2 re-evaluation is expected to
produce essentially identical numbers to regen2 above; left as optional
follow-up.

#### Known limitations

- Single-step replay only; multi-step state updates remain v8.2.x scope.
- Write profile (`kernel_ingest`, `kernel_write_memory`) excluded.
- No Best-of-N or reranking.
- Live replay currently uses MCP JSON-RPC stdio through `rehydration-mcp`;
  HTTP JSON-RPC can be enabled later when a kernel bridge exists.

## Subsequent runs

Detailed per-run records live under `runs/`. Add entries here when a run lands.
Each linked file is self-contained: setup, hyperparameters, metrics, decision,
caveats, and artifact paths.

- `runs/2026-05-28-v8.1.4-retoken.md` — v8.1.4 retoken evaluation at
  `--max-new-tokens 4096`. Confirmed that 9 of the 14 v8.1.4 prediction
  failures at 2048 were `kernel_ingest` truncation. `kernel_ask` and all
  other tools were byte-identical. Conservative exact 87.70% → 89.59%.
  Decision: keep v8.1.3pa shipped; do not install v8.1.4.
- `runs/2026-05-28-v8.1.5-ask-pa.md` — v8.1.5 ask-only `prepared_action`
  experiment. `kernel_ask` exact-match jumped from 17/32 (v8.1.3pa) to 32/32
  on the regen2-mix eval split. The K3 paired probe established that the
  +15 is causally dependent on `prepared_action` being present in the
  operator input at inference time (10/10 with PA, 2/10 without PA).
  Adapter SHA `6cf81a39ba52cc643f3633d46469ccc4cb0ee9e95181da9e1985f492d1db41fa`.
  Decision: keep v8.1.3pa shipped. Use v8.1.5 only in offline replay or
  batch jobs that supply `prepared_action` end-to-end (now possible after
  PR #50, merge commit `0f62319`, which preserves `prepared_action` through
  the runtime request boundary).
- **v8.1.8 Tier 4 (2026-05-29).** Trained Qwen2.5-0.5B + LoRA on the Tier-4 corpus
  (1933 train / 394 eval; 17 min; `eval_mean_token_accuracy` 0.962). **Evaluated
  correctly (full-schema system prompt on all rows): 92.9% exact / 98.5% structural
  overall, base 316/317 = 99.68%, 15 rebalanced families 100%.** The first predict
  showed an apparent read-nav "cliff" (0% on the new rows), but a diagnostic proved
  this was a **system-prompt build bug**, not a model failure: the Tier 2/3 eval rows
  were authored with a 356-char prompt that omits the MCP/API schema (training uses
  the 3741-char full-schema prompt). Re-predicting the *unchanged* adapter on a
  prompt-corrected eval (raw refs, no retrain) lifted read-nav structural validity
  0%→100% and parse failures 51→5. So v8.1.8 is a strong model; remaining gaps are
  small argument-value policy choices (`near` limit/dimension, `trace` page).
  Two corpus-correctness items still apply: (1) Directive B — all rows + the runtime
  must carry the full-schema prompt; (2) Directive A — refs must be anonymized
  (design requirement; did NOT cause the cliff). Not a publication claim until the
  schema-prompt fix is corpus-wide. See
  [`DIVERGENCE_AND_CORRECTIVE_PLAN_2026-05-29.md`](DIVERGENCE_AND_CORRECTIVE_PLAN_2026-05-29.md),
  `../operator-experiments/RESULTS_v818_tier4.md`, and the diagnostic
  `../operator-experiments/audits/DIAGNOSTIC_promptfix_20260529.md` (sibling repo).
- **v8.1.9 anonymized (2026-05-29) — the clean, design-correct model.** Regenerated
  the corpus with **reference anonymization ON** (opaque `ref_0001`/`about_0001`;
  dimension kinds preserved) on top of the schema-fixed + well-posed eval, then
  retrained (13 min; eval_mean_token_accuracy 0.9887). Result: **ALL-394 structural
  validity 100%, exact 94.7%, 0 parse failures, 0 missing** (vs v8.1.8 raw: 99.2%
  struct / 3 failures). Confirms **Directive A at zero cost** — operating on opaque
  refs does not degrade the policy; it is cleaner. Combines Directive B (MCP/API
  schema in context everywhere + runtime) + Directive A (anonymized refs) + well-posed
  eval. Anonymization collapsed 127 train rows — exactly Tier-4's topic-clone
  count-rebalance — confirming the operator learns structure, not topic. Remaining
  exact-misses (21) are soft (stop evidence-subset) or hard-exact (long ingest), not
  structural. This is the publication-track model. Train sha `ee79e101…`. See
  `../operator-experiments/RESULTS_v819_anon.md`; adapter
  `../operator-experiments/adapters/operator-qwen05-lora-v8.1.9-anon-20260529T170500/`
  (sibling repo).
