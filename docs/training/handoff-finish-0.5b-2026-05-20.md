# Handoff — finish training the 0.5B Qwen operator (2026-05-20)

> **Historical context only (banner 2026-05-29).** Superseded by the realistic-v7
> plan and v7.3 closure. Use [`runbook.md`](runbook.md) and
> [`model-history.md`](model-history.md) for current guidance, and
> [`DIVERGENCE_AND_CORRECTIVE_PLAN_2026-05-29.md`](DIVERGENCE_AND_CORRECTIVE_PLAN_2026-05-29.md)
> for the current training-design truth.

> Status update after PR #27:
>
> Do **not** use this handoff as a direct training recipe anymore. PR #27
> replaced the stale v5/v6 transition with a contract-v6 gate, and the user
> explicitly rejected GPU training on fixture-grade data because it would
> inflate the score. The next interpretable run must use the realistic-v7
> corpus described in
> [`operator-realistic-corpus-v7-plan-2026-05-20.md`](operator-realistic-corpus-v7-plan-2026-05-20.md).
> The historical instructions below remain as context for what existed before
> the decision, not as the current execution plan.

Briefing for the next agent picking up the operator-0.5B training run.
Read this end-to-end before doing anything; the architectural
constraints are non-negotiable and were the reason the previous attempt
was stopped.

## 1. Goal

Train `Qwen/Qwen2.5-0.5B-Instruct` + LoRA on the v5 KMP/MCP
conformance dataset, predict on the holdout, score with the strict
operator action contract, and decide pass/fail against a 90% exact-match
gate (or whatever the user chooses). The previous run (v4) topped out at
**24.1% exact-action accuracy** — see
`/tmp/kernel-operator-qwen05-conformance-full-v4-policy-eval.json`. v5
adds the dataset cleanups the postmortem demanded; we re-run the same
pipeline against the cleaner data using the **new operator repo**.

## 2. Where to do the work

**Authoritative repo:** `/home/tirso/ai/developents/operator`
(independent git history, independent Cargo workspace, own CI surface).

**NOT authoritative:**
- `/home/tirso/ai/developents/rehydration-kernel/legacy/operator/` —
  the quarantined pre-disaster 21 crates. They keep compiling so the
  internal evidence remains reproducible; **do not import from them,
  do not extend them, do not copy patterns from them.**
- `/home/tirso/ai/developents/rehydration-kernel/k8s/kernel-operator-*`
  — 96 dataset-specific job manifests. Reference values only; the
  operator repo ships generic templates at `k8s/qwen05-lora-*.yaml`
  that you should use instead.

## 3. What previously broke the architecture

Read `rehydration-kernel/docs/product/operator-architecture-postmortem-2026-05-18.md`
for the full diagnosis. The short version, with the rule each failure
produced:

| Failure in pre-disaster code               | Rule now in force                                       |
| ------------------------------------------ | ------------------------------------------------------- |
| 800–2200 line files                        | ADR 0002: one public type per file                      |
| `tool: &str`, `cursor_key: &str` params    | ADR 0003: typed `KernelTool` + `Cursor` value objects   |
| `serde_json::Value` in domain/app          | ADR 0004: no serde_json/toml in domain or application   |
| Trajectory builders with 5+ responsibilities | Hexagonal + DDD; ports in `*-application`, adapters in `*-infra` |
| Benchmark logic leaking into operator      | Benchmark adapters belong to the kernel, never operator |
| `replay-cli → rehydration-mcp` forbidden edge | Bounded-context edges enforced by architecture tests   |
| Patches over symptoms instead of closed design | Specification pattern for validators; composition over inheritance |

If you find yourself reaching for `serde_json::Value` inside
`operator-*-domain` or `operator-*-application`, or for a `&str` tool
name, **stop and revisit the design**. The architecture tests in
`crates/operator-architecture-tests/` will fail anyway, but the point
is to not write that code in the first place.

## 4. Where the architecture is documented

- `docs/architecture/operator/README.md` — index.
- `00-principles.md`, `01-bounded-contexts.md`, `02-design-patterns.md`,
  `03-dependency-injection.md` — global rules.
- `10-shared-context.md`, `20-synthetic-context.md`, `30-evaluation-context.md`,
  `40-replay-context.md`, `50-training-context.md` — per-context detail.
- `decisions/0001-…` through `decisions/0012-…` — ADRs.

Skim the README and the ADRs; do not read everything end-to-end yet.

## 5. What's already built (do not rebuild)

**Six CLIs**, each in `crates/operator-*-cli/`:

| Bin                          | What it does                                                                 | Smoke tests                                            |
| ---------------------------- | ---------------------------------------------------------------------------- | ------------------------------------------------------ |
| `operator-train`             | Orchestrates train → predict → validate via process invokers                 | `operator-training-cli/tests/cli_smoke.rs`             |
| `operator-policy-eval`       | Scores `predictions.jsonl` against ground-truth JSONL under the strict contract | `operator-evaluation-cli/tests/cli_smoke.rs`           |
| `operator-replay`            | Replays predicted actions against a live KMP MCP endpoint                    | `operator-replay-cli/tests/cli_smoke.rs`               |
| `operator-synthesize`        | Fixture-grade trajectory JSONL (one per `KmpMcpCapability`, cloned N times)  | `operator-synthetic-cli/tests/cli_smoke.rs`            |
| `operator-contract-coverage` | Audits a trajectory JSONL for per-tool coverage + contract validity          | `operator-evaluation-cli/tests/contract_coverage_smoke.rs` |
| `operator-llm-baseline`      | Calls a frontier LLM over the same SFT JSONL to measure the ceiling          | `operator-evaluation-cli/tests/llm_baseline_smoke.rs`  |

**Python pipeline** at `scripts/operator/`:
- `prepare_operator_sft_dataset.py` — trajectory JSONL → `openai_{train,eval,all}.jsonl`.
- `train_operator_sft_lora.py` — LoRA SFT trainer.
- `predict_operator_sft.py` — runs the trained adapter on a holdout.
- `audit_operator_sft_no_gold.py`, `compare_operator_policy_details.py`,
  `deanonymize_operator_predictions.py` — auxiliary tools.

**K8s job templates** at `k8s/`:
- `qwen05-lora-train.yaml` — fine-tunes 0.5B + LoRA on a single GPU.
- `qwen05-lora-predict.yaml` — runs the trained adapter on a holdout.
- `README.md` — env-var overrides + cluster prerequisites.

**Authoritative end-to-end recipe:** `docs/training/runbook.md`. Read
it. The agent who picked up the previous attempt skipped this and
re-invented half the pipeline.

## 6. On-disk artifacts you can reuse (pre-disaster output)

These survived the redesign and are still valid wire inputs:

```
/tmp/kernel-operator-conformance-full-v5/
    trajectories.jsonl                # raw trajectory JSONL, v5

/tmp/kernel-operator-conformance-full-v5-sft/
    openai_all.jsonl                  # 58 rows, system/user/assistant
    openai_eval.jsonl                 # 14 rows holdout
    eval_trajectories.jsonl           # ground-truth TrainingTrajectoryDto for the holdout
    all_trajectories.jsonl            # ground-truth for the full set
```

The 58/14 split is small but it's the dataset the v4 baseline used, so
results are comparable. The SFT JSONL is ready to feed the trainer
directly — **do not re-run `prepare_operator_sft_dataset.py`** unless
you have a reason; the cleaned-v5 prep already happened pre-disaster.

The v4 LoRA adapter lives at
`/tmp/kernel-operator-qwen05-lora-conformance-full-v4/`. Keep it for
comparison; do not overwrite.

## 7. Concrete plan

1. **Sanity-check the workspace.**
   ```
   cd /home/tirso/ai/developents/operator
   cargo fmt --all -- --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```
   Everything must be green before you touch anything. As of
   2026-05-20 it is — 362 tests pass.

2. **Audit the v5 trajectory JSONL.**
   ```
   cargo run --release -p operator-evaluation-cli --bin operator-contract-coverage -- \
       --trajectories /tmp/kernel-operator-conformance-full-v5/trajectories.jsonl \
       --require-zero-invalid
   ```
   If any row violates the strict contract, **fix the upstream
   generator, not the row.** The previous attempt patched outputs;
   that's what got us here.

3. **Confirm the SFT prep is consistent.** Sample a row from
   `/tmp/kernel-operator-conformance-full-v5-sft/openai_all.jsonl` and
   verify the assistant content's `action` is the same shape the
   strict `OperatorActionDto` mapper accepts. If not, re-run the
   prep:
   ```
   python scripts/operator/prepare_operator_sft_dataset.py \
       --input /tmp/kernel-operator-conformance-full-v5/trajectories.jsonl \
       --output-dir /tmp/operator-sft-v5
   ```

4. **Train.** GPU is on the existing K8s cluster; image and hostPath
   defaults in `k8s/qwen05-lora-train.yaml` assume the operator repo is
   at `/home/tirso/ai/developents/operator`. Edit the manifest env
   vars only if the dataset path differs; do not edit the script.
   ```
   kubectl apply -f k8s/qwen05-lora-train.yaml
   kubectl wait --for=condition=complete --timeout=2h \
       job/operator-qwen05-lora-train
   ```
   Training writes the adapter under `OUTPUT_DIR` (default
   `/tmp/operator-qwen05-lora`). Do **not** point `OUTPUT_DIR` at the
   v4 adapter directory; archive v4 first if you need a clean slot.

5. **Predict.**
   ```
   kubectl apply -f k8s/qwen05-lora-predict.yaml
   kubectl wait --for=condition=complete --timeout=1h \
       job/operator-qwen05-lora-predict
   ```
   Output goes to `/tmp/operator-qwen05-predictions/`:
   `predictions.jsonl`, `summary.json`, `failures.jsonl`.

6. **Score under the strict contract.**
   ```
   cargo run --release -p operator-evaluation-cli --bin operator-policy-eval -- \
       --predictions  /tmp/operator-qwen05-predictions/predictions.jsonl \
       --ground-truth /tmp/kernel-operator-conformance-full-v5-sft/eval_trajectories.jsonl \
       --min-pass-rate 0.9
   ```
   Exit 0 = pass at 90% exact-match; exit 1 = fail. Per-tool /
   per-mode metrics are printed regardless.

7. **(Optional) compare against a frontier baseline.**
   ```
   cargo run --release -p operator-evaluation-cli --bin operator-llm-baseline -- \
       --sft        /tmp/kernel-operator-conformance-full-v5-sft/openai_eval.jsonl \
       --output     /tmp/operator-baseline-gpt4o-v5 \
       --api-base   https://api.openai.com/v1 \
       --api-key-file /tmp/openai.txt \
       --model      gpt-4o-mini
   cargo run --release -p operator-evaluation-cli --bin operator-policy-eval -- \
       --predictions  /tmp/operator-baseline-gpt4o-v5/predictions.jsonl \
       --ground-truth /tmp/kernel-operator-conformance-full-v5-sft/eval_trajectories.jsonl
   ```
   The frontier baseline costs paid API calls. Use `--limit 5` first
   to sanity-check the wiring before turning it loose on all 14
   rows.

## 8. Things that will trip you up

- **The synthetic generator is fixture-grade.** Do not use
  `operator-synthesize` to produce training data for the real run.
  It exists for plumbing / smoke purposes only. Use the kernel-emitted
  v5 trajectories at `/tmp/kernel-operator-conformance-full-v5/`.
- **API keys.** Anthropic key at `/tmp/claude.txt`; OpenAI key at
  `/tmp/openai.txt`. Never commit them; never echo them; never put
  them in any summary or log line. The `operator-llm-baseline` CLI
  reads them via `--api-key-file` and they never appear in `summary.json`.
- **vLLM lives at `https://llm.underpassai.com`** with mTLS ingress, not
  a port-forward. The CLI takes that URL via `--api-base` and the
  cluster ingress handles the TLS handshake.
- **E2E tests cost money.** The LLM baseline hits a paid endpoint per
  row. Always pre-check config (`--limit 1`) before a full holdout
  run.
- **GitHub token at `/tmp/github.txt`.** The `gh` CLI reads it
  automatically; do not paste it into prompts.
- **Do not skip git hooks** (`--no-verify`, `--no-gpg-sign`) — they
  enforce signing and prevent accidental secret leaks.

## 9. PRs in flight you may need to merge first

As of 2026-05-20:
- **PR #24** `feat/operator-llm-baseline` — adds the
  `operator-llm-baseline` bin. Includes the quality-audit followup
  (sync_all + extra tests). Merge to `main` before doing §7.7.

Check `gh pr list` for newer PRs. Merge anything green before
starting; rebase on `main` if your branch falls behind.

## 10. What "done" looks like

- v5 training job has succeeded in K8s.
- `operator-policy-eval` reports a verdict against the holdout.
- The verdict is one of: PASS at 90%, PASS at a lower threshold the
  user agreed to, or FAIL with per-tool / per-mode metrics that show
  where the 0.5B falls short.
- The `TrainingManifest` TOML, the LoRA adapter, the `predictions.jsonl`,
  and the `policy-eval.json` are archived together so the run is
  reproducible.
- A one-line entry is appended to `docs/training/model-history.md`
  noting the dataset hash, the verdict, and the date.

That last step is the publication-grade gate — only do it when the
user explicitly approves.

## 11. If you get stuck

- Re-read `docs/training/runbook.md` first. It is the authoritative
  recipe and was already updated through PR #24.
- The Rust use cases that the CLIs wire are exercised by unit tests in
  each `*-application` crate. If a CLI behaviour surprises you, grep
  the use case before guessing.
- Architecture tests in `crates/operator-architecture-tests/tests/`
  enforce the rules from §3. If you trip one, **fix your code**, do
  not weaken the test.
- If you genuinely think an architectural rule is wrong for this
  task, write an ADR proposing the change before bending the rule.
  The user will not accept silent loosening.

Good luck.
