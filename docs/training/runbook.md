# Operator training runbook

End-to-end recipe for training a small operator policy (Qwen 0.5B +
LoRA SFT) and validating it before declaring a release candidate.
This runbook assumes the holdout dataset has already been produced
upstream (synthetic generation or external benchmark adapter — the
fixture-grade `operator-synthesize` CLI exists for contract gates;
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

## Reference anonymization policy (MANDATORY)

> Read this before generating any corpus. A 2026-05-29 forensic audit found the
> v7/v8 training path diverged from the intended design by shipping
> **un-anonymized domain refs**. See
> [`DIVERGENCE_AND_CORRECTIVE_PLAN_2026-05-29.md`](DIVERGENCE_AND_CORRECTIVE_PLAN_2026-05-29.md).

The operator "only learns to use KMP" — it must operate on **opaque** refs, never
on teacher domain topics. Model-facing refs MUST be anonymized to `ref_0001` /
`about_0001` (kernel `kernel-tool-operator-model-plan.md:182-186`). The V6 holdout
built this way reached 1.000 exact action accuracy. This is a **design
requirement** (don't ship a model that memorized domain content); note the
2026-05-29 diagnostic showed anonymization did NOT cause the v8.1.8 read-nav cliff
(that was a system-prompt build bug — see below), so anonymization is enforced for
correctness, not as a measured performance fix.

> **Equally important — the MCP/API schema must be in the system prompt on EVERY
> row.** The v8.1.8 read-nav "cliff" was caused by Tier 2/3 eval rows using a
> 356-char prompt that omitted the tool schema (training uses the 3741-char
> full-schema prompt). All SFT rows — and the runtime `DEFAULT_SYSTEM_PROMPT` —
> must carry the canonical full-schema prompt. See
> [`DIVERGENCE_AND_CORRECTIVE_PLAN_2026-05-29.md`](DIVERGENCE_AND_CORRECTIVE_PLAN_2026-05-29.md).

Enforced in `prepare_operator_sft_dataset.py`:

- `--anonymize-refs` defaults **ON** (`--no-anonymize-refs` is for de-anonymized
  replay/debug only — never a release-candidate corpus). Historically it defaulted
  OFF, which was the divergence.
- `looks_like_ref()` covers all domain-topic prefixes via `DOMAIN_TOPIC_REF_PREFIXES`
  (`incident:`/`migration:`/`bug:`/`product:`/`docs:`/`about:`/`evidence:`/`question:run:`/`turn:run:`);
  KMP dimension kinds (`agent:`/`task:`/`topic:`/`session:`/`attempt:`) are structural
  and intentionally preserved.
- A prep-time guard (`assert_no_domain_refs_in_pairs`) **fails the build** if any
  model-facing user/assistant field still carries a domain-topic ref.
- The corpus manifest records `anonymize_refs` and `anonymization_guard`.

No model trained with anonymization OFF is publishable. v8.1.8 (and earlier v8.x)
were trained un-anonymized and must be regenerated + retrained.

## Training data policy

There are two different corpus classes:

| Corpus | Purpose | Allowed for release-candidate training? |
| --- | --- | --- |
| contract-v6 fixture corpus | pipeline/contract gate, 10/10 tool coverage, no-gold and round-trip smoke | no |
| realistic-v7 corpus | episode-based operator decisions with teacher-backed or hand-authored realistic processes | yes, once gates pass |

Do not spend GPU time on contract-v6 expecting an interpretable model score. It
is intentionally fixture-grade and will inflate results. Use it only to prove
that prepare/train/predict/eval wiring is sound.

The realistic-v7 path has two different sizes: 300-600 rows is only a training
smoke; an interpretable baseline needs at least 1500-3000 realistic rows and a
mandatory frontier ceiling on the held-out episode split.

The next interpretable training run waits for
[`operator-realistic-corpus-v7-plan-2026-05-20.md`](operator-realistic-corpus-v7-plan-2026-05-20.md).

## Artifact storage policy

`/tmp/` is **staging only**. As of 2026-05-29, every artifact that has any
post-session value lives under
`/home/tirso/ai/developents/operator-experiments/`. The runbook examples
below still emit to `/tmp/...` because the k8s `hostPath` volumes mount
`/tmp` into the pods, but the moment a run completes (or a corpus, audit,
adapter, prediction, or k8s manifest is worth keeping) it is copied into
the archive at:

```
/home/tirso/ai/developents/operator-experiments/
├── INDEX.md                # version table + locator
├── builders/               # *.py corpus build scripts (one per version)
├── corpora/                # generated SFT JSONL + manifest.json per version
├── audits/                 # corpus diagnostic scripts + reports
├── adapters/               # trained LoRA weight directories
├── probes/                 # prediction outputs + paired-probe artifacts
├── k8s-jobs/               # rendered Job YAMLs actually applied to the cluster
└── docs/                   # standalone analysis notes
```

Rules:

- Never reference `/tmp/...` paths in commit messages, PRs, manifests, or
  closure docs as if they were durable. Quote the
  `operator-experiments/...` path instead.
- Run artifacts MUST be timestamped in their directory name (e.g.
  `operator-sft-v8.1.8-tier3-20260529T003900`) and copied into the
  archive before `/tmp/` is reaped.
- Adapters live under `adapters/` only after a predict run has validated
  them; raw checkpoints from a failed run are not archived.
- Corpora carry a `manifest.json` with the train/eval SHA256 and the
  per-row rationale for every appended row. Without the manifest the
  corpus is not promoted to `operator-experiments/corpora/`.
- The archive is the source of truth for `model-history.md` references
  and for downstream training rebuilds.

If `/tmp` is full or being cleaned, the missing input is always
re-derivable from the archive (corpus + adapter + manifest are enough to
re-run predict and re-validate). This is the single reason `/tmp` may be
treated as ephemeral without losing reproducibility.

## Field wiring: `prepared_action` vs `requested_*`

This section is the canonical reference for which subject fields are
actually plumbed end-to-end and therefore safe to use as training-time
signals. **Do not propose `requested_*` as a fix for any operator
inference gap until you read this.**

### Short version

`prepared_action` **is** runtime-wired. `requested_*` **is not**.

Phrase precisely: "`requested_*` is not runtime-wired" — not "it does
not exist". It exists in the system prompt and in one legacy script
flag. It does not exist in the typed DTO, the typed domain, the
mappers, the runtime use case, or any deployed corpus.

### Why this gap is easy to trip on

Three sources of confusion that produce repeated wasted proposals:

1. The Operator system prompt mentions `requested_*` literally:
   `If visible_state contains requested_wake, requested_ask, …` — that
   line tells the model how it would respond if such fields were present.
2. `scripts/operator/prepare_operator_sft_dataset.py` accepts
   `--inject-target-request-fields`, which DOES write `requested_*` into
   `visible_state`. This flag is intended for translation/replay smoke
   tests only and is NOT part of the canonical SFT data path.
3. The names sound natural — "to fix `kernel_ask`, populate
   `requested_ask`" looks like a clean per-tool hint API.

None of these survive a wiring audit (see recipe below).

### End-to-end wiring contrast

| Layer | `requested_*` | `prepared_action` |
| --- | --- | --- |
| Typed in `VisibleStateDto` (`operator-shared-contract`) | no — fields are only `known_refs`, `known_dimensions`, `active_cursor`, `budget` | n/a — `prepared_action` lives top-level on `CalibrationSubjectDto`, not inside visible_state |
| Typed in `CalibrationSubjectDto` | no — DTO has no `requested_*` field | yes — `pub prepared_action: Option<OperatorActionDto>` |
| Typed in `CalibrationSubject` domain | no | yes — `Option<PreparedOperatorAction>` |
| Round-tripped by `CalibrationSubjectMapper` | no | yes — DTO↔domain both directions |
| Emitted by runtime (`build_subject_from_request`) | no | yes — since PR #50 (`Preserve prepared_action through runtime request boundary`) |
| Populated in deployed corpora (v8.1.2-sft-v2, v8.1.3pa, v8.1.5) | 0 rows | v8.1.3pa: write rows; v8.1.5: write + ask rows |
| Causal evidence in eval | none measured | K3 paired probe on v8.1.5 ask: 10/10 with PA, 2/10 without PA (`runs/2026-05-28-v8.1.5-ask-pa.md`) |

`prepared_action` won the design fight. It is a single top-level typed
field that carries the full action the operator should emit (when
upstream knows it). `requested_*` was a planned per-tool distributed
hint API that was never typed or plumbed.

### Why this matters for training decisions

If a corpus row populates `requested_*` and the model learns to react to
it, the production runtime will never send that field, and the eval lift
will not materialize on the live endpoint. Same failure mode as
proposing any other signal the production path does not produce.

By contrast, `prepared_action` has a documented production gap (no
upstream planner today sets it; the runtime now propagates it from
PR #50 if a future caller supplies it), but every other layer is wired,
so the gap is one well-scoped architectural decision rather than a
multi-layer rebuild.

### Hard rule

- Do not add new `requested_*` fields to a corpus, schema, mapper, or
  runtime path.
- Do not propose adding them to close an inference gap. The K3-style
  probe template from v8.1.5 plus `prepared_action` is the canonical
  mechanism for any "give the operator a hint" experiment.
- If a future component genuinely needs per-tool hints distributed
  across visible_state (rather than one consolidated action), that is a
  schema decision — file it as a typed DTO change with mappers and a
  runtime read path, do not freelance with magic field names.

### Wiring-audit recipe

When you are tempted to use a field as a training signal, walk these
five checks. If any is empty, the field is not runtime-wired and must
not be used as a benchmark-decision signal.

```bash
# 1. Typed in any DTO?
rg -nE "^\s*pub\s+<field>" operator/crates/operator-shared-contract/src/

# 2. Typed in domain?
rg -nE "^\s*<field>" operator/crates/operator-shared-domain/src/visible_state/

# 3. Read by mappers (DTO ↔ domain)?
rg "<field>" operator/crates/operator-synthetic-infra/src/mappers/

# 4. Emitted by runtime (does it reach the policy)?
rg "<field>" operator/crates/operator-runtime-application/

# 5. Populated in the deployed corpus (sample any active openai_eval.jsonl)?
jq '.messages[1].content | fromjson | .visible_state | has("<field>")' \
   <corpus.jsonl> | sort | uniq -c
```

Run the recipe before claiming "the model should learn `<field>`". For
`prepared_action` post-PR #50, all five checks return non-empty. For
`requested_*` all five return empty in the canonical SFT pipeline.

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

## 0c. Build the P0.4/P0.5 contract-v6 corpus

Use the contract-v6 builder when the goal is to regenerate the current
operator-native corpus and SFT from the typed contract:

```bash
bash scripts/operator/build_contract_v6_corpus.sh
```

Defaults:

```text
artifact root:      ../rehydration-kernel-artifacts/operator
minimum examples:  4 per KMP/MCP tool
split:             group by about
eval groups:       one about per tool
```

The builder writes generated JSONL outside the repo by default. It gates the
run in this order:

```text
operator-synthesize
  -> contract coverage on source trajectories
  -> prepare SFT
  -> contract coverage on train trajectories
  -> contract coverage on eval trajectories
  -> no-gold audit
  -> trainer validate-only
  -> predictor validate-only
  -> round_trip_smoke against the generated SFT
```

This corpus is still contract-grade synthetic data, not the final realistic
teacher-generated dataset. Its purpose is to close P0.4/P0.5 and prevent GPU
training from starting until the complete KMP/MCP action surface round-trips.
It must not be used as the release-candidate corpus.

## 0d. Build the realistic-v7 corpus

Use the realistic-v7 path when the goal is to create the first
teacher-backed corpus that is eligible for an interpretable SFT baseline.

First generate deterministic scenarios outside the repo:

```bash
python3 scripts/operator/build_realistic_scenarios.py \
    --output ../rehydration-kernel-artifacts/operator/scenarios-v2/scenarios.jsonl \
    --count 1650 \
    --seed 42
```

The scenario builder is deterministic and does not call an LLM. It uses
handcrafted templates plus structural variation knobs. At the end it calls
`operator-realistic-corpus --validate-only` so malformed scenarios fail before
any paid teacher call is possible.

Then verify the semantic acceptance rules before spending money:

```bash
python3 scripts/operator/verify_scenarios_v2.py \
    ../rehydration-kernel-artifacts/operator/scenarios-v2/scenarios.jsonl
```

The verifier checks unique about ids, at least 100 `writer_pre_read` scenarios,
at least 50 `full` scenarios, full target coverage, five themes and
non-instructional happy goals for non-write targets. It also verifies that
`stop` and `kernel_goto` templates carry strict semantic acceptance criteria
for `stop.reason` and `goto.cursor.kind`.

Before a paid smoke, run the deterministic regression pack. This is not a
random first-30 sample; it replays diagnosed scenarios by id and uses the same
realistic corpus use case, semantic acceptance gate and drop sink as production:

```bash
cargo run --release -p operator-synthetic-cli --bin operator-regression-pack-v7 -- \
    --scenarios ../rehydration-kernel-artifacts/operator/scenarios-v2/scenarios.jsonl \
    --pack docs/training/regression_pack_v7.txt \
    --output ../rehydration-kernel-artifacts/operator/regression-pack-v7-smoke \
    --api-base https://api.openai.com/v1 \
    --api-key-file /tmp/openai.txt \
    --prompt crates/operator-synthetic-infra/prompts/teacher_calibration_v5.md \
    --model gpt-4o-mini \
    --temperature 0.0
```

For local no-cost wiring checks, add `--mock-teacher`. If the real run drops a
row, `dropped.jsonl` includes the parsed `predicted_action`, `subject_hash` and
`teacher_finish_reason`, so the failure can be diagnosed without another paid
call.

Then run a paid 30-row smoke:

```bash
OPERATOR_RUN_LIMIT=30 \
OPERATOR_PROMPT=crates/operator-synthetic-infra/prompts/teacher_calibration_v5.md \
  bash scripts/operator/build_realistic_v7_corpus.sh
```

Only after the smoke is green, run the full corpus:

```bash
OPERATOR_PROMPT=crates/operator-synthetic-infra/prompts/teacher_calibration_v5.md \
bash scripts/operator/build_realistic_v7_corpus.sh
```

The v7 builder gates the run in this order:

```text
verify_scenarios_v2
  -> operator-realistic-corpus
  -> contract coverage on accepted trajectories
  -> prepare SFT
  -> no-gold audit
  -> frontier ceiling with gpt-4o-mini
  -> oracle round-trip smoke
```

By default the run writes to:

```text
../rehydration-kernel-artifacts/operator/<run-id>/
```

Important environment overrides:

| Variable | Meaning |
| --- | --- |
| `OPERATOR_RUN_ID` | output directory name |
| `OPERATOR_ARTIFACT_ROOT` | root for external artifacts |
| `OPERATOR_SCENARIOS` | scenario JSONL path |
| `OPERATOR_RUN_LIMIT` | `0` for full, otherwise smoke limit |
| `OPERATOR_MODEL` | teacher/frontier model, default `gpt-4o-mini` |
| `OPERATOR_PROMPT` | required teacher prompt path, currently calibration prompt v5 |
| `OPERATOR_API_KEY_FILE` | token file, default `/tmp/openai.txt` |

Do not raise `--max-drop-rate` to make a run pass. The production gate is 5%.
If the smoke or full run fails, inspect `dropped.jsonl` and `report.json`, fix
the scenario templates if they are wrong, regenerate a new scenario version and
rerun.

After the full run, inspect the frontier ceiling. A useful v7.3 corpus should
land below near-perfect performance; `75%..92%` overall accuracy is the target
sanity range. If the ceiling is `95%+`, treat the corpus as still too
instructional and rework the goals before training.

## 1. Prepare the SFT dataset

The Python `prepare_operator_sft_dataset.py` script turns a
trajectory-v1 JSONL into the conversational SFT shape the trainer
consumes:

```bash
python scripts/operator/prepare_operator_sft_dataset.py \
    --trajectories /tmp/trajectories.jsonl \
    --output /tmp/operator-sft
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
    --batch-size 8 \
    --max-new-tokens 2048 \
    --temperature 0.0 \
    --stop-after-json \
    --force
```

## 4. Score and gate (Rust validation loop)

Use `operator-policy-eval` (§4b) for offline scoring. The Rust
training application also exposes the full validate-trained-run use
case for callers that want to invoke the predictor and scorer behind
one application boundary:

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

Both the `is_passing(...)` projection and the underlying counts live on
`ValidateTrainedRunOutcome` so callers can log + gate from a single
return value.

## 5. Promote or iterate

- **Pass**: archive the dataset hash, the LoRA adapter weights, and
  the `summary.json` + evaluation report next to the
  `TrainingManifest` TOML the build phase wrote (the
  `TomlManifestWriter` records dataset provenance + readiness
  gates). Copy the timestamped corpus dir into
  `operator-experiments/corpora/`, the adapter into
  `operator-experiments/adapters/`, and the predictions directory into
  `operator-experiments/probes/` (see "Artifact storage policy" above
  for the full layout). Add a one-line entry to
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
    --ground-truth /tmp/trajectories.jsonl \
    --min-pass-rate 0.9
```

The binary joins predictions and ground truth by `step_id`, runs
`EvaluateOperatorPolicyUseCase` under the strict KMP contract, prints
per-tool / per-mode metrics, and exits 0 or 1 based on the
`--min-pass-rate` threshold (omit the flag to print metrics only).
Useful for inspecting a historical run, comparing against a different
contract version, or gating a publication candidate.

It also prints `stop_decision_match` (count/rate): of the stop ground-truths, the
fraction the model also stopped on **for the same reason**, ignoring the `evidence`
subset and `answer` text. Exact-match over-penalizes `stop` because the evidence
subset is under-determined by the visible state (any grounded subset is defensible)
and is not contract-checked; `stop_decision_match` is the faithful stop-policy metric.

**Eval determinism.** `predict_operator_sft.py` defaults to `--sort-by-length`,
which groups similar-length prompts into each batch so padding is uniform and batch
composition is independent of input order — this makes batched temp-0 inference
reproducible run-to-run (without it, ~4% of long `kernel_ingest`/`kernel_write_memory`
rows flip predictions between runs from batch re-composition). For bit-exact
determinism regardless, use `--batch-size 1`.

## 4c. Or replay predictions against a live KMP with `operator-replay`

If you have a running kernel MCP JSON-RPC endpoint, you can execute
each predicted action against the real KMP and measure tool-call
success rate / failure modes:

```bash
cargo run --release -p operator-replay-cli --bin operator-replay -- \
    --predictions  /tmp/operator-qwen05-predictions/predictions.jsonl \
    --ground-truth /tmp/trajectories.jsonl \
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
    --sft        /tmp/operator-sft/openai_eval.jsonl \
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
   behaviour. The realistic-v7 corpus plan defines the next accepted
   training-data path.
2. **CLI parity with the pre-disaster kernel is complete.**
   `operator-policy-eval` covers prediction scoring (§4b),
   `operator-replay` covers live KMP execution (§4c),
   `operator-contract-coverage` covers dataset audits (§0b), and
   `operator-llm-baseline` covers frontier-LLM ceilings (§4d). The
   only outstanding work is the teacher-model-backed synthetic
   generator listed above.

## Running the Operator SFT pipeline

The Operator SFT pipeline is two Kubernetes Jobs in `k8s/`. Both use hostPath
volumes for the repo and `/tmp`, so they run on a single-node cluster with local
GPUs.

### Prerequisites

- Single-node Kubernetes cluster with NVIDIA GPU device plugin
- 4x compute capability 8.0+ GPUs (Ampere or newer), ~24GB VRAM each
- `/tmp/huggingface/` writable (HF cache externalized)
- Input dataset at `/tmp/<DATASET_DIR>/openai_train.jsonl` +
  `openai_eval.jsonl` (produced by `scripts/operator/prepare_operator_sft_dataset.py`
  from a corpus `trajectories.jsonl`)

### Step 1 — Train SFT LoRA

```bash
kubectl -n underpass-runtime apply -f k8s/qwen05-lora-train.yaml
kubectl -n underpass-runtime get jobs operator-qwen05-lora-train -w
kubectl -n underpass-runtime logs -f job/operator-qwen05-lora-train
```

Defaults: reads from `/host-tmp/operator-sft/`, writes to
`/host-tmp/operator-qwen05-lora/`. To override, set `DATASET_DIR` and
`OUTPUT_DIR` in the manifest before applying, or render the manifest with local
environment overrides.

What it does:

- 4 GPUs via `torchrun --nproc_per_node=4` DDP
- Qwen2.5-0.5B-Instruct + LoRA r=16, lr 2e-4 cosine, 3 epochs, bf16
- Effective batch 16 (4 GPUs x batch 4 x grad_accum 1)
- ~12 min wall clock on the WRX80 4x RTX 3090 node
- Outputs final adapter + 3 epoch checkpoints + tokenizer

### Step 2 — Predict over holdout

```bash
kubectl -n underpass-runtime apply -f k8s/qwen05-lora-predict.yaml
kubectl -n underpass-runtime get jobs operator-qwen05-lora-predict -w
kubectl -n underpass-runtime logs -f job/operator-qwen05-lora-predict
```

Defaults: reads `/host-tmp/operator-sft/openai_eval.jsonl`, loads
`/host-tmp/operator-qwen05-lora/`, and writes
`/host-tmp/operator-qwen05-predictions/`. For versioned runs, override
`DATASET_JSONL`, `ADAPTER_DIR`, and `OUTPUT_DIR` before applying.

What it does:

- 1 GPU, base model + LoRA loaded
- `predict_operator_sft.py` over eval JSONL, batch 8
- Free JSON generation with `--max-new-tokens 2048`, `--temperature 0.0`, and
  `--stop-after-json`
- ~5-15 min for 242 eval rows
- Outputs `predictions.jsonl` + `summary.json` + `failures.jsonl`

### Step 3 — Score predictions

```bash
cargo run --release -p operator-evaluation-cli --bin operator-policy-eval -- \
  --predictions /tmp/operator-qwen05-predictions/predictions.jsonl \
  --ground-truth ../rehydration-kernel-artifacts/operator/<corpus>/trajectories.jsonl
```

With PR #43 shape-violation handling, predictions with invalid action shape
count as `contract_valid=false` instead of aborting the run. Invalid JSON rows
still abort because no action or `step_id` can be trusted.

### Reproducibility

Each Job's `summary.json` records:

- dataset path and selected row count
- `model_id` and adapter path for predict
- generation controls (`max_new_tokens`, `temperature`, `stop_after_json`)
- timestamps or completion status from the Kubernetes Job

The chain `{dataset_sha -> train output -> adapter_sha -> predict output ->
policy_eval report}` is reproducible from these.

### Caveats — v8.0 vintage

- The predictor does **not** use constrained decoding. Output is free JSON
  generation with `--stop-after-json` only. This is a v8.1 backlog item.
- Comparison vs a frontier model, for example `gpt-4o-mini`, is apples-to-apples
  only when both predict without structured-output strict. Document this
  precision explicitly in any closure doc.
