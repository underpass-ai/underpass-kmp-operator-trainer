# v7.3 Smoke Investigation Evidence — 2026-05-23

## Executive summary

The latest smoke did not fail because the kernel contract suddenly changed. It
failed because we compared non-equivalent runs and because the previous smoke
gate was too weak to prove the semantics we thought it proved.

Two process gaps are now explicit:

1. A teacher adapter change invalidated prior smoke evidence. PR #37 changed
   the teacher execution surface through structured output and prepared-action
   fast-path behavior. Calibration was rerun, but the realistic corpus smoke was
   not treated as mandatory evidence before attempting the next paid path. That
   was insufficient.
2. `--limit 30` is not a targeted regression test. It truncates the first 30
   scenarios from the JSONL. The scenarios are deterministic, but the generation
   uses seeded variation knobs for refs, abouts, dimensions and budgets. This is
   useful for corpus diversity; it is not the right tool for diagnosing specific
   known failures.

## Runs compared

### Previous green smoke

Artifact:
`../rehydration-kernel-artifacts/operator/realistic-v7-smoke-fix1-20260522T163917Z/`

Report:

```text
scenarios_sha256: 3a9feab2858a2fbe4f59c809ca313e3fb20fb8bc15deccee92ba366885f5a6ea
prompt_sha256:    87e26adf71049c165daa68ea016091846f576b9d4902de5276ce37e81956913c
total:            30
accepted:         29
dropped:          1
drop_rate:        0.03333333333333333
gate_passed:      true
dropped_by_reason:
  contract_violation: 1
```

Drop:

```json
{"scenario_id":"scenario:kernel_near:candidate-refs:0020","target":"near","reason":"contract_violation","message":"ContractViolations { items: [ContractViolation { code: UnknownMemoryRef, field: \"near.anchor\", message: \"memory ref 'bug_investigation:candidate-refs:metric:000' is not in visible state\" }] }"}
```

### Latest failed smoke

Artifact:
`../rehydration-kernel-artifacts/operator/realistic-v7-20260522T215932Z/`

Report:

```text
scenarios_sha256: 789b063f1e9db77d8016d2390e0eb16a8d83777c6602e8183222cf05ec3f8e47
prompt_sha256:    87e26adf71049c165daa68ea016091846f576b9d4902de5276ce37e81956913c
total:            30
accepted:         28
dropped:          2
drop_rate:        0.06666666666666667
gate_passed:      false
gate_reason:      drop_rate 0.0667 > max_drop_rate 0.0500
dropped_by_reason:
  target_mismatch: 2
```

Drops:

```json
{"scenario_id":"scenario:kernel_inspect:after-near:0007","target":"inspect","reason":"target_mismatch","message":"expected inspect, got kernel_goto"}
{"scenario_id":"scenario:stop:no-candidate:0028","target":"stop","reason":"target_mismatch","message":"expected stop, got kernel_ask"}
```

## Why the runs are different

The two runs used the same prompt hash but not the same scenario artifact:

```text
previous green smoke scenarios_sha256: 3a9feab2858a2fbe4f59c809ca313e3fb20fb8bc15deccee92ba366885f5a6ea
latest failed smoke scenarios_sha256:  789b063f1e9db77d8016d2390e0eb16a8d83777c6602e8183222cf05ec3f8e47
```

The previous green smoke used `scenarios-v2`. The latest failed smoke used
`scenarios-v3`.

### Scenario artifact differences

`scenarios-v2`:

```text
rows:                 1650
bad known_ref rows:   1650
bad known refs:       6600
mode distribution:    read=1250, write=250, writer_pre_read=100, full=50
```

`scenarios-v3`:

```text
rows:                 1650
bad known_ref rows:   0
bad known refs:       0
mode distribution:    read=1250, write=250, writer_pre_read=100, full=50
```

This means `v3` fixed a real scenario-quality problem: refs now use the `about:`
shape consistently. But it also means the previous smoke is not comparable as
evidence for the current dataset.

## The two failing scenarios changed materially

### `scenario:kernel_inspect:after-near:0007`

In `scenarios-v2`:

```text
target: kernel_inspect
mode:   read
budget: calls_remaining=1, tokens_remaining=600
goal:   Near expansion returned candidate refs; the metadata needed to choose lives on visible node node:technical_incident:after-near:mitigation:000.
refs:
  node:technical_incident:after-near:mitigation:000
  node:technical_incident:after-near:hypothesis:000
  node:technical_incident:after-near:fix:000
  node:technical_incident:after-near:deployment:000
```

In `scenarios-v3`:

```text
target: kernel_inspect
mode:   read
budget: calls_remaining=8, tokens_remaining=4000
goal:   Near expansion returned candidate refs; the metadata needed to choose lives on visible node about:incident:checkout-errors:kernel_inspect:after-near:case-000:node:hypothesis:000.
refs:
  about:incident:checkout-errors:kernel_inspect:after-near:case-000:node:hypothesis:000
  about:incident:checkout-errors:kernel_inspect:after-near:case-000:node:fix:001
  about:incident:checkout-errors:kernel_inspect:after-near:case-000:node:deployment:002
  about:incident:checkout-errors:kernel_inspect:after-near:case-000:node:root-cause:003
```

Observed result in the latest smoke:

```text
expected inspect, got kernel_goto
```

Interpretation: the v3 subject has a fuller, more realistic ref and a much
larger budget. The phrase "visible node" plus a full `about:...:node:...` ref
can plausibly bias the teacher toward navigation (`kernel_goto`) instead of
node detail read (`kernel_inspect`). That is a data/teacher policy ambiguity,
not a wire-format failure.

### `scenario:stop:no-candidate:0028`

In both v2 and v3 the scenario is a stop/no-candidate template with
`calls_remaining=1`.

In `scenarios-v3`:

```text
target: stop
mode:   read
budget: calls_remaining=1, tokens_remaining=600
goal:   Tools have been exhausted on this about; remaining budget would not produce a ref that changes the answer.
refs:
  about:bug:ios-login-loop:stop:no-candidate:case-000:node:risk:000
  about:bug:ios-login-loop:stop:no-candidate:case-000:node:fix:001
  about:bug:ios-login-loop:stop:no-candidate:case-000:node:state:002
  about:bug:ios-login-loop:stop:no-candidate:case-000:node:owner:003
```

Observed result in the latest smoke:

```text
expected stop, got kernel_ask
```

This matches the calibration limitation already diagnosed: with
`calls_remaining=1`, `kernel_ask` is contract-valid. The strict budget spec only
rejects tool calls at `calls_remaining=0`; it does not encode the policy
"remaining budget would not produce a useful ref."

## Why the previous smoke looked green anyway

The previous green smoke accepted `scenario:stop:no-candidate:0028`, but the
accepted action was not semantically correct for the template:

```json
{
  "kind": "stop",
  "reason": "answer_ready",
  "answer": "Tools have been exhausted on this about; remaining budget would not produce a ref that changes the answer.",
  "evidence": ["bug:timezone-reporting:stop:no-candidate:case-000"]
}
```

The target was `stop`, and the action was a `Stop`, so production accepted it.
But the template is `no-candidate`; a stricter semantic check would have expected
`stop(no_candidate)`, not `stop(answer_ready)`.

The reason is in the target matcher:

- `crates/operator-synthetic-domain/src/case/synthetic_generation_target.rs:85-90`
  checks only whether the action is the right tool/kind.
- For `Stop`, it accepts any `OperatorAction::Stop(_)`.
- It does not check `StopReason`.
- For `kernel_goto`, it accepts any `kernel_goto`, regardless of cursor variant.

Relevant code:

```rust
pub fn matches_action(self, action: &OperatorAction) -> bool {
    match self {
        Self::Kmp(capability) => action.tool() == Some(capability.tool()),
        Self::Stop => matches!(action, OperatorAction::Stop(_)),
        Self::Escalate => matches!(action, OperatorAction::Escalate(_)),
    }
}
```

Therefore the previous smoke was green at the tool/kind level, not at the full
semantic target level.

## Additional findings from the investigation

### Finding 1 — the previous green smoke had hidden semantic misses

The previous green smoke did not only miss `stop(no_candidate)`. A local audit
of accepted rows found another semantic mismatch:

```text
realistic-v7-smoke-fix1-20260522T163917Z

scenario:kernel_goto:temporal-cursor:0021
  expected semantic shape: temporal-ish cursor
  accepted action:        kernel_goto cursor.kind=ref

scenario:stop:no-candidate:0028
  expected semantic shape: stop(no_candidate)
  accepted action:        stop(answer_ready)
```

The latest failed smoke still accepted the temporal-cursor mismatch:

```text
realistic-v7-20260522T215932Z

scenario:kernel_goto:temporal-cursor:0021
  expected semantic shape: temporal-ish cursor
  accepted action:        kernel_goto cursor.kind=ref
```

This is the same root pattern as the `stop` issue: production target matching
operates at action kind / tool granularity. It does not validate template-level
semantics such as stop reason or cursor variant.

### Finding 2 — calibration is stricter than realistic corpus matching

Calibration compares the teacher action against `accepted_actions` and stores
debug evidence when it fails. For example, the calibration report includes:

```text
case_id:          calib:software_migration:stop-no-candidate
expected:         stop(no_candidate)
predicted:        kernel_ask
tool_matched:     false
contract_valid:   true
```

Realistic corpus production does not have equivalent accepted actions. It only
knows the broad generation target:

```text
target: stop
```

That means:

- Calibration can distinguish `stop(no_candidate)` from `stop(answer_ready)`.
- Realistic corpus currently cannot; both are accepted as `Stop`.
- Calibration can distinguish cursor variants if the accepted action says so.
- Realistic corpus currently cannot; any `kernel_goto` satisfies a `goto` target.

This is not a bug in the strict contract validator. It is a gap in the corpus
production acceptance gate.

### Finding 3 — observability exists, but is not sufficient for root cause

PR #37 added useful observability, but the current event surface is still too
thin for failure diagnosis.

The latest run directory contains only:

```text
dropped.jsonl
report.json
trajectories.jsonl
```

For drops, the persisted row is:

```json
{"scenario_id":"scenario:kernel_inspect:after-near:0007","target":"inspect","reason":"target_mismatch","message":"expected inspect, got kernel_goto"}
```

What is missing:

- full `predicted_action`
- predicted action arguments
- raw assistant content
- finish reason
- structured-output parse metadata
- subject snapshot hash or inline subject used for that row

The code path confirms the loss:

- `crates/operator-synthetic-infra/src/adapters/stderr_progress_sink.rs:80-95`
  emits only `index`, `scenario_id`, `target`, and `reason_kind` for drops.
- `crates/operator-synthetic-infra/src/adapters/jsonl_streaming_sink.rs:61-68`
  writes a mapped drop DTO, not the full teacher output.
- `crates/operator-synthetic-infra/src/mappers/realistic_corpus_report_mapper.rs:49-55`
  serializes only `scenario_id`, `target`, `reason`, and `message`.

Result: for `target_mismatch`, we know "expected inspect, got kernel_goto" but
we do not know which cursor/ref was produced unless we rerun a paid call. That
is exactly the kind of observability gap this PR was supposed to reduce.

### Finding 4 — `--limit 30` amplifies whichever templates happen to appear early

The scenario generator interleaves templates deterministically. The first 30
rows include one `kernel_inspect:after-near` and one `stop:no-candidate`.

If those two templates fail, the smoke drop rate is:

```text
2 / 30 = 6.67%
```

In the full corpus the same two templates appear 25 times each:

```text
50 / 1650 = 3.03%
```

So the same systematic issue can fail the 30-row smoke but fit under the 1650
drop gate. That is not wrong mathematically; it means the 30-row smoke is a
coarse operational check, not a stable statistical estimate.

### Finding 5 — v2 was not a trustworthy baseline for scenario quality

`scenarios-v2` had 1650/1650 rows with known refs that did not start with
`about:`. It still produced a green smoke after one fix, but that green result
was over a corpus artifact that violated the ref-shape invariant later enforced
by `scenarios-v3`.

Therefore:

- v2 green smoke cannot be used as evidence that v3 should pass.
- v2 green smoke cannot be used as evidence that stop/goto semantics were right.
- v3 is the better artifact, but it exposed new policy/semantic ambiguity.

### Finding 6 — teacher adapter changes should invalidate realistic smoke evidence

The teacher execution surface changed after the earlier smoke evidence:

```text
3adf061 Enforce structured teacher output
da79b9c Add corpus observability to structured teacher output
ebc5d9b Close prepared-action EOF calibration gap
```

Even with the same model and prompt hash, these changes alter the request/parse
surface:

- `response_format: json_schema` constrains generation differently from free
  JSON.
- `max_completion_tokens` / timeout behavior affects large outputs.
- prepared-action fast-path bypasses the LLM for prepared writes.

Conclusion: after those commits, old realistic smoke results should have been
declared stale. Passing calibration was necessary but not sufficient.

### Finding 7 — the contract validator is doing what it says, but policy is missing

For `stop:no-candidate`, the model's `kernel_ask` is contract-valid when
`calls_remaining=1`.

`BudgetAllowsActionSpec` rejects tool calls only when no calls remain. It does
not encode the higher-level policy "this question has no useful candidate, so
stop instead of asking one more time."

That policy can live in a future semantic corpus gate or policy specification,
but it is not currently represented by the strict action contract.

## Why changing the teacher should have forced a new realistic smoke

The "teacher" is not only the model name and prompt. The adapter behavior is
part of the teacher surface. PR #37 changed that surface:

- `3adf061` added structured teacher output.
- `da79b9c` added corpus observability and touched the teacher adapter.
- `ebc5d9b` added the prepared-action fast-path.

The calibration run did exercise the teacher adapter after those changes, and it
passed at `34/36` match with `36/36` contract-valid and `0` shape failures.

That was useful but insufficient because calibration and realistic corpus smoke
measure different things:

| Gate | What it measures | What it misses |
| --- | --- | --- |
| Calibration | 36 handcrafted cases with accepted actions | Full scenario generator distribution, scenario wording, per-template repetition |
| Realistic smoke | First N generated scenarios through production pipeline | If `N=30`, it is not representative or targeted |
| Full run | All 1650 scenarios | Expensive; should not be first place to discover deterministic template failures |

Honest conclusion: after changing structured output / fast-path behavior, the
previous realistic smoke evidence should have been considered stale. A new
realistic smoke was needed before any full paid run. We did eventually run it,
but the process did not make it an explicit invalidation rule.

## Are the scenarios random?

They are deterministic, not random in the "changes every run" sense.

`scripts/operator/build_realistic_scenarios.py:637-645` builds scenarios by:

```python
template = templates[index % len(templates)]
variation = index // len(templates)
rng = random.Random(f"{seed}:{template.target}:{template.slug}:{variation}")
scenarios.append(render_scenario(template, index, variation, rng))
```

That means:

- Same seed + same code = same JSONL.
- The values inside each scenario still come from seeded variation knobs.
- The output order is deterministic.

The production use case applies `--limit` by truncating the JSONL:

`crates/operator-synthetic-application/src/use_cases/build_realistic_corpus_use_case.rs:50-53`

```rust
let mut scenarios = self.scenarios.read()?;
if let Some(limit) = limit {
    scenarios.truncate(limit.as_usize());
}
```

So `--limit 30` means "first 30 rows", not "30 cases selected to test the known
failure modes" and not "stratified sample across all templates."

That explains the frustration: we are trying to close specific failures, but the
smoke gate is currently a production sample shortcut. It can expose new issues,
but it is not a precise regression test for known issues.

## Template repetition risk

Both failed templates appear 25 times in the full v3 corpus:

```text
kernel_inspect / after-near: 25 rows
stop / no-candidate:        25 rows
```

If both templates fail systematically:

```text
50 / 1650 = 3.03% drop rate
```

That is below the 5% production drop gate, but it leaves little margin for any
other systematic template failure.

The current first-30 smoke contains exactly one row from each failed template:

```text
first 30 failed-template rows: 2
first 30 drop rate if both fail: 2 / 30 = 6.67%
```

This is why the smoke failed even though those two templates alone would not
necessarily fail the full 1650 gate.

## What should change before another paid full run

### 1. Add a targeted regression pack

Known failures should be tested by explicit scenario IDs/templates, not by
hoping they appear in `--limit 30`.

Minimum pack:

```text
scenario:kernel_inspect:after-near:0007
scenario:stop:no-candidate:0028
all kernel_goto cursor templates
all stop reason templates
all known calibration misses carried forward
```

This pack should run whenever the teacher adapter, prompt, scenario generator,
or target matcher changes.

### 2. Keep realistic smoke, but make it stratified

`--limit 30` should not be the only smoke. A better smoke should include at
least one row per target/template class or a deterministic stratified selection.

### 3. Add semantic checks for accepted rows

The current drop gate only checks tool/kind. It does not catch:

- `stop(no_candidate)` becoming `stop(answer_ready)`
- `stop(budget_exhausted)` becoming another stop reason
- `kernel_goto` choosing a ref cursor for a temporal/trace cursor template

These should be audited separately. This does not mean changing the operator's
runtime behavior. It means the corpus-production gate needs a stricter semantic
audit for scenario targets.

### 4. Persist full predicted action for drops

`dropped.jsonl` currently stores only:

```text
scenario_id, target, reason, message
```

For `target_mismatch`, that loses the actual action arguments. Calibration
reports have richer debug fields (`predicted_action`, `accepted_actions`);
realistic corpus drops should have equivalent observability.

### 5. Do not treat observability as complete until drops are self-contained

The observability slice is not complete for production debugging until every
drop row can be understood without repeating a paid request. Minimum additional
fields:

```text
predicted_action
subject_summary or subject_sha256
teacher_finish_reason
raw_content_tail on parse failure
```

This is especially important before full runs, because otherwise a 1650-row run
can still leave us with aggregate counts and incomplete root-cause evidence.

## Disposition

Do not run the full 1650 paid run from the current state.

The current evidence says:

- The latest smoke failure is real.
- The previous green smoke was not strong enough evidence.
- The scenario generator is deterministic but not a targeted regression test.
- The teacher adapter change should invalidate prior smoke evidence.
- We need targeted regression + semantic accepted-row audit before spending on
  another full run.
