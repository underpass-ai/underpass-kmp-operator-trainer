# Operator v7.3 smoke gap analysis - 2026-05-22

This document records the gaps found while running the first real paid v7.3
smoke against `gpt-4o-mini` with `teacher_calibration_v5.md`.

Scope: 30-row smoke only. This is not the 1650-row full run and it does not
close v7.3 by itself.

## Evidence

Artifacts:

- Failed smoke: `../rehydration-kernel-artifacts/operator/realistic-v7-smoke-20260522T163535Z`
- Passing corpus smoke: `../rehydration-kernel-artifacts/operator/realistic-v7-smoke-fix1-20260522T163917Z`
- Scenario artifact: `../rehydration-kernel-artifacts/operator/scenarios-v2/scenarios.jsonl`
- Prompt: `crates/operator-synthetic-infra/prompts/teacher_calibration_v5.md`
- Model: `gpt-4o-mini`
- Temperature: `0.0`

## Smoke 1 result

Run id: `realistic-v7-smoke-20260522T163535Z`

| Metric | Value |
| --- | --- |
| total_scenarios | 30 |
| accepted_count | 25 |
| dropped_count | 5 |
| drop_rate | 16.67% |
| max_drop_rate_gate | 5% |
| gate_passed | false |

Dropped by reason:

| Reason | Count |
| --- | --- |
| teacher_error | 3 |
| target_mismatch | 1 |
| contract_violation | 1 |

Failures:

| Scenario | Expected target | Actual/failure | Root cause |
| --- | --- | --- | --- |
| `kernel_ask:evidence-query` | `kernel_ask` | teacher chose `kernel_inspect` | Ask goal was too close to "inspect this visible node"; it did not say the missing fact should be retrieved as a bounded memory query. |
| `kernel_near:relations` | `kernel_near` | invalid near shape: empty `dimensions` | Near goal did not make the dimension explicit enough for the teacher to preserve it. |
| `kernel_near:writer-pre-read-target-candidates` | `kernel_near` | unknown `near.anchor` | Writer-pre-read near goal let the teacher use the about/scope as anchor instead of a visible memory ref. |
| `kernel_ask:clarify-choice` | `kernel_ask` | invalid action JSON: kind `kernel_ask` | Ask case was semantically valid but prompt/goal pressure led the teacher to emit tool name as action kind. This is caught by strict DTO parsing. |
| `kernel_near:candidate-refs` | `kernel_near` | invalid near shape: empty `dimensions` | Same near under-specification pattern as `kernel_near:relations`. |

Conclusion: smoke 1 found data-shaping gaps in ask/near templates. It did not
justify softening the drop gate or changing the teacher prompt. The correct fix
was to strengthen the scenario generator and verifier.

## Fix after smoke 1

Applied fixes:

- ask goals now describe a missing deterministic memory fact/query, without
  naming the tool;
- near goals now expose a visible anchor ref and a visible dimension in the
  situational text;
- writer-pre-read near goals now use explicit visible candidate refs;
- `verify_scenarios_v2.py` now rejects any `kernel_near` scenario whose goal
  does not mention at least one visible ref and one visible dimension.

This keeps the option C rule intact: goals remain situational, not
instructional. We did not add "call kernel_near" or similar tool-leading text.

## Smoke 2 result

Run id: `realistic-v7-smoke-fix1-20260522T163917Z`

| Metric | Value |
| --- | --- |
| total_scenarios | 30 |
| accepted_count | 29 |
| dropped_count | 1 |
| drop_rate | 3.33% |
| max_drop_rate_gate | 5% |
| gate_passed | true |

Dropped by reason:

| Reason | Count |
| --- | --- |
| contract_violation | 1 |

Remaining dropped row:

```text
scenario:kernel_near:candidate-refs:0020
```

The teacher chose `bug_investigation:candidate-refs:metric:000` as
`near.anchor`; the visible ref was `node:bug_investigation:candidate-refs:metric:000`.
This is a strict contract failure (`UnknownMemoryRef`) caused by losing the
`node:` prefix.

Impact: acceptable for the smoke because drop-rate is below 5%, but this is a
watch item for the full run. If many `near` drops appear in the full run, the
template should make the full ref form harder to shorten.

## Downstream pipeline gaps found

The corpus gate passed, then the downstream pipeline exposed three tooling
gaps. These were pipeline contract gaps, not corpus quality failures.

### Gap 1 - SFT prep rejected `escalate`

Failure:

```text
unsupported model-facing action kind `escalate`
```

Cause: `prepare_operator_sft_dataset.py` still treated the model-facing action
space as `tool_call` plus `stop`. v7 data includes `escalate` as a first-class
operator action.

Fix:

- added `escalate` examples to read/full system prompts;
- added `tool:escalate` capability tracking;
- allowed `escalate` in model-facing action validation.

Impact if unfixed: SFT prep would fail on any realistic corpus containing
escalation. The model would either never learn escalation or the pipeline would
force us to remove valid rows.

### Gap 2 - OpenAI SFT JSONL dropped `step_id`

Failure: `operator-llm-baseline` could generate predictions, but
`operator-policy-eval` and the prediction path require stable `step_id`
metadata to join predictions to ground truth.

Cause: `write_openai_jsonl()` emitted only `messages`, losing the row id.

Fix: `openai_train.jsonl` and `openai_eval.jsonl` now retain:

```json
{"step_id":"...","messages":[...]}
```

Impact if unfixed: frontier ceiling could not be scored reliably. We would
have generation output but no auditable policy-eval join.

### Gap 3 - Predictor validator used old modes

Failure:

```text
mode_unsupported:full
```

Cause: `predict_operator_sft.py` still recognized the old mode set:
`read`, `write_context_read`, `write`. It did not accept current v7 modes
`full` and `writer_pre_read`.

Related stale contract: `prepare_operator_sft_dataset.py` also had the old
writer-pre-read profile with `kernel_trace` and without `kernel_wake` /
`kernel_ask`. The Rust domain contract says `OperatorMode::WriterPreRead`
allows exactly `wake`, `ask`, `near`, `inspect`.

Fix:

- predictor now accepts `full`;
- predictor now accepts `writer_pre_read`;
- historical `write_context_read` remains as compatibility alias only;
- writer-pre-read prompt/profile now matches the Rust domain contract.

Impact if unfixed: v7.3 semantic closure would be impossible because spec C
requires `full` and `writer_pre_read` rows.

## Downstream gate results after fixes

Using run id `realistic-v7-smoke-fix1-20260522T163917Z`:

| Gate | Result |
| --- | --- |
| corpus generation gate | pass: 29/30 accepted, drop-rate 3.33% |
| contract coverage | pass: 10/10 tools, 0 invalid |
| mode coverage in smoke | `read=19`, `write=4`, `writer_pre_read=4`, `full=2` |
| no-gold audit | pass: 0 findings over 29 rows |
| SFT prep | pass: train=25, eval=4 |
| train validate-only | pass |
| predict validate-only | pass |
| oracle policy eval | pass: 4/4 exact-match |

Frontier ceiling generation also succeeded:

| Metric | Value |
| --- | --- |
| selected eval rows | 4 |
| succeeded | 4 |
| failed | 0 |

Frontier policy-eval on the 4-row eval split:

| Metric | Value |
| --- | --- |
| exact_match | 0/4 |
| tool_match | 4/4 |
| contract_valid | 4/4 |

Interpretation: the frontier ceiling from this smoke is not semantically
decisive because the eval split has only 4 rows. It does show that the frontier
selects the correct tool and emits contract-valid actions, but exact arguments
vary. The meaningful 75%-92% ceiling criterion must be evaluated on the full
1650-row run with a real eval split.

## Current state

The smoke is good enough to proceed to the full run:

- structural verifier passes on 1650 generated scenarios;
- Rust `--validate-only` accepts the 1650-row JSONL;
- paid 30-row corpus smoke passes the 5% drop-rate gate;
- downstream pipeline gates now run end-to-end on the accepted smoke rows;
- all fixes preserve strict validation instead of softening gates.

## Risks before full run

| Risk | Signal to watch in full run | Required action if it appears |
| --- | --- | --- |
| teacher shortens refs, especially `node:` prefixes | `contract_violation` dominated by `near.anchor` or target refs | strengthen templates around full ref spelling; do not relax contract |
| ask still flips to inspect | `target_mismatch` dominated by `kernel_ask` | audit ask templates; clarify missing fact/query situation |
| frontier ceiling too high | full-run exact-match `>=95%` | goals still leak tool/action semantics; re-craft goals |
| frontier ceiling too low | full-run exact-match `<75%` | goals too opaque; inspect failing families |
| drop-rate above 5% | full report `gate_passed=false` | fix scenario templates or visible_state consistency; do not raise max-drop-rate |

## Full-run preconditions

Before launching the 1650-row run:

1. regenerate `scenarios-v2/scenarios.jsonl` from the final script version;
2. run `verify_scenarios_v2.py`;
3. run `operator-realistic-corpus --validate-only`;
4. keep `teacher_calibration_v5.md`, `gpt-4o-mini`, `temperature=0.0`;
5. keep `--max-drop-rate 0.05`.

v7.3 closes only after the full run passes corpus gate, contract coverage,
no-gold audit, frontier ceiling sanity and oracle round-trip smoke.
