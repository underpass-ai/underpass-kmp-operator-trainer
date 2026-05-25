# v8.0 SFT closure audit — frontier/predict blockers

Date: 2026-05-23  
Branch: `feature/operator-v8-0-sft-kickoff`  
Local commit created during closure follow-up: `a649469 Add v8.0 SFT training entry to model-history`

## Summary

The v8.0 SFT checkpoint exists and is loadable as an artifact, but closure is blocked before trained-model evaluation:

1. The retroactive frontier ceiling run completed all 242 eval rows with 0 API failures, but the official scorer fails while parsing frontier predictions because at least one predicted `kernel_ingest` action is contract-invalid.
2. The local trained-model predictor does not support constrained decoding. Running unconstrained prediction would violate the PR #42 hard rule and would not be apples-to-apples with the frontier/teacher schema.

No additional training was run. No cluster changes were made during this follow-up.

## Artifacts

### Dataset

- Eval JSONL: `/tmp/operator-sft-v8.0/openai_eval.jsonl`
- Eval rows: 242
- Eval SHA-256: `626eec90c827296c405d75b2395316e3dfe3370ea6fd3d6934906427ec403212`

### Trained adapter

- Adapter dir: `/tmp/operator-qwen05-lora-v8.0/`
- Adapter file: `/tmp/operator-qwen05-lora-v8.0/adapter_model.safetensors`
- Adapter SHA-256: `4a5ed6fa2057cb2f20db3289fc51ae114ad32167c4f13db1cfb68a3c8855f7b1`

### Ground truth corpus

- Trajectories: `../rehydration-kernel-artifacts/operator/realistic-v7-full-v5-1-pr41-20260523T101100Z/trajectories.jsonl`
- Trajectories SHA-256: `3b9194f3fc14eb90228662cab3c0a71008b1375e57b0064a2021cd9df416e51e`

### Frontier ceiling output

- Output dir: `../rehydration-kernel-artifacts/operator/frontier-ceiling-v8.0-20260523T140341Z/`
- `predictions.jsonl`: 83,131 bytes
- `failures.jsonl`: 0 bytes
- `summary.json`: 315 bytes

Summary:

```json
{
  "predictor": "operator-llm-baseline-v1",
  "api_base": "https://api.openai.com/v1",
  "model": "gpt-4o-mini",
  "temperature": 0.0,
  "dataset": "/tmp/operator-sft-v8.0/openai_eval.jsonl",
  "selected": 242,
  "succeeded": 242,
  "failed": 0,
  "started_at_unix": 1779545031,
  "finished_at_unix": 1779545703
}
```

## Frontier scoring blocker

Command run:

```bash
cargo run --release -p operator-evaluation-cli --bin operator-policy-eval -- \
  --predictions ../rehydration-kernel-artifacts/operator/frontier-ceiling-v8.0-20260523T140341Z/predictions.jsonl \
  --ground-truth ../rehydration-kernel-artifacts/operator/realistic-v7-full-v5-1-pr41-20260523T101100Z/trajectories.jsonl
```

Result:

```text
policy-eval failed: read predictions: predictions reader 'jsonl_predictions_reader' shape violation at line 15: action: tool 'kernel_ingest' arguments shape is invalid: ingest.provenance.observed_at must be RFC3339: premature end of input
```

The failed row is:

```text
line: 15
step_id: scenario:kernel_ingest:missing-provenance:1074:step:1074
```

Frontier prediction excerpt:

```json
{
  "step_id": "scenario:kernel_ingest:missing-provenance:1074:step:1074",
  "action": {
    "kind": "tool_call",
    "tool": "kernel_ingest",
    "arguments": {
      "about": "about:bug:ios-login-loop:kernel_ingest:missing-provenance:case-018",
      "dry_run": true,
      "idempotency_key": "...",
      "provenance": {
        "observed_at": "...",
        "source_agent": "...",
        "source_kind": "agent",
        "correlation_id": "...",
        "causation_id": "..."
      }
    }
  }
}
```

Ground truth for the same step has a valid timestamp:

```json
"observed_at": "2026-05-22T00:00:00+00:00"
```

## Scope of invalid frontier ingest predictions

Read-only scan over `predictions.jsonl` found 12 `kernel_ingest` predictions with invalid placeholder `observed_at: "..."`:

| Line | step_id | observed_at |
| ---: | --- | --- |
| 15 | `scenario:kernel_ingest:missing-provenance:1074:step:1074` | `...` |
| 16 | `scenario:kernel_ingest:missing-provenance:1131:step:1131` | `...` |
| 17 | `scenario:kernel_ingest:missing-provenance:0732:step:0732` | `...` |
| 18 | `scenario:kernel_ingest:missing-provenance:1017:step:1017` | `...` |
| 90 | `scenario:kernel_ingest:anemic-fallback:0938:step:0938` | `...` |
| 92 | `scenario:kernel_ingest:declared-dimensions:1423:step:1423` | `...` |
| 93 | `scenario:kernel_ingest:declared-dimensions:1195:step:1195` | `...` |
| 94 | `scenario:kernel_ingest:declared-dimensions:1252:step:1252` | `...` |
| 177 | `scenario:kernel_ingest:missing-provenance:1302:step:1302` | `...` |
| 178 | `scenario:kernel_ingest:missing-provenance:1473:step:1473` | `...` |
| 179 | `scenario:kernel_ingest:missing-provenance:1416:step:1416` | `...` |
| 187 | `scenario:kernel_ingest:after-pre-read:1235:step:1235` | `...` |

This means the frontier run is not scoreable by the strict evaluator as-is. The evaluator is correctly rejecting invalid predictions rather than silently counting them.

## Non-official diagnostic counts

These are not official results because `operator-policy-eval` stopped before producing a report.

Computed by direct JSON comparison against the PR #41 trajectories:

```text
total:           242
raw_exact:       90 / 242 = 37.19%
kind/tool match: 242 / 242 = 100.00%
```

Per target:

| Target | Total | Raw exact | Kind/tool match |
| --- | ---: | ---: | ---: |
| `escalate:beyond_capability` | 6 | 6 | 6 |
| `kernel_ask` | 30 | 9 | 30 |
| `kernel_forward` | 35 | 0 | 35 |
| `kernel_goto` | 3 | 3 | 3 |
| `kernel_ingest` | 19 | 0 | 19 |
| `kernel_inspect` | 15 | 14 | 15 |
| `kernel_near` | 18 | 0 | 18 |
| `kernel_rewind` | 13 | 0 | 13 |
| `kernel_trace` | 23 | 0 | 23 |
| `kernel_wake` | 58 | 58 | 58 |
| `kernel_write_memory` | 22 | 0 | 22 |

Interpretation: the frontier model is selecting the right action kind/tool for every eval row, but frequently fails exact argument copying, especially for structured payload actions. This is a frontier-ceiling evaluation issue that should be handled explicitly before using the number as a training ceiling.

## Trained predictor blocker

The available predictor does not support constrained decoding.

Command:

```bash
python scripts/operator/predict_operator_sft.py --help
```

Supported relevant flags:

```text
--dataset-jsonl
--model-id
--adapter
--output
--torch-dtype
--limit
--batch-size
--max-new-tokens
--temperature
--stop-after-json
--resolve-prepared-payloads
--validate-only
--force
```

Missing flags required by the PR #42 plan:

- `--constrained-decoding`
- `--schema-source`
- `--base-model`
- `--lora-checkpoint`

The closest existing flag is `--stop-after-json`, but that only stops after the first complete JSON object. It is not constrained decoding and does not enforce the same schema used by the teacher/frontier path.

## Current phase status

| Phase | Status | Evidence |
| --- | --- | --- |
| Phase 0 — Pre-flight | Done earlier | Dataset/model/GPU were verified before training. |
| Phase 1 — Dataset prep | Done | `/tmp/operator-sft-v8.0/openai_train.jsonl` and `/tmp/operator-sft-v8.0/openai_eval.jsonl`. |
| Phase 2 — Frontier ceiling | Blocked at scoring | 242/242 API predictions succeeded, but `operator-policy-eval` rejects invalid `kernel_ingest` prediction shape. |
| Phase 2.5.1 — Training observability patch | Skipped in original run | Documented in `model-history.md`. |
| Phase 2.5.2 — Observer agent | Skipped in original run | Documented in `model-history.md`. |
| Phase 3 — SFT training | Done | `/tmp/operator-qwen05-lora-v8.0/adapter_model.safetensors`, final eval_loss 0.0235070214. |
| Phase 4 — Predict + eval | Not run | Blocked because constrained decoding is not implemented in the predictor. |
| Phase 5 — Decision | Not reached | Needs scoreable frontier ceiling and constrained trained-model prediction. |

## Git state

Local commit created:

```text
a649469 Add v8.0 SFT training entry to model-history
```

Remaining untracked file:

```text
docs/training/viability_pack_gpt55.txt
```

No push was performed.

## Recommended next decision

There are two separate blockers:

1. Frontier ceiling scorer behavior: decide whether invalid frontier predictions should be counted as `contract_valid=false` rows rather than aborting the whole report. That requires an evaluation-path change, not another paid run.
2. Trained prediction path: implement or authorize a real constrained decoding path for `predict_operator_sft.py` before evaluating the LoRA checkpoint. Running `--stop-after-json` would be useful as a diagnostic only, not as the official v8.0 comparison.

