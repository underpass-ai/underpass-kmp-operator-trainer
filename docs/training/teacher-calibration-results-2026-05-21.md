# Teacher Calibration Results 2026-05-21

This note records the first real v7.2.5 teacher calibration runs. It is
evidence for PR #32, not a training approval.

## Scope

The calibration runner measures whether a frontier teacher can choose the next
bounded Operator action from a visible KMP/MCP subject.

The runner intentionally compares against human-authored accepted actions before
v7.3 generates realistic corpus data. A technically valid action is not enough:
the action must match the intended Operator policy for the case.

## Artifacts

External runtime artifacts live outside the repo:

```text
../rehydration-kernel-artifacts/operator/calibration-cases-v1/cases.jsonl
../rehydration-kernel-artifacts/operator/calibration-cases-v2/cases.jsonl
../rehydration-kernel-artifacts/operator/calibration-cases-v3/cases.jsonl
../rehydration-kernel-artifacts/operator/calibration-runs/
```

Committed prompts:

```text
crates/operator-synthetic-infra/prompts/teacher_calibration_v1.md
crates/operator-synthetic-infra/prompts/teacher_calibration_v2.md
```

`teacher_calibration_v2.md` is a stricter structured-action prompt produced
after v2 showed that v1 was too weak for exact KMP/MCP argument preservation.
It is not evidence that the gate passed.

## Verified Fixes

Two implementation fixes landed before the later runs:

- HTTPS transport now uses `reqwest` with `rustls-tls`.
- Failed case reports include both `predicted_action` and `accepted_actions`,
  so the dataset can be audited without rerunning the model.

Both fixes are required. Without the second one, exact-match failures are hard
to classify as teacher error, dataset ambiguity, or contract mismatch.

## Dataset Iterations

### v1

The first dataset exposed a legitimate exact-match false negative for
`kernel_ask.query`. The teacher selected the correct tool and produced a
contract-valid action, but the query wording differed from the single accepted
string.

Decision: keep exact-match as the primary gate, but use multiple accepted
actions for narrative fields only.

Allowed narrative fields:

- `kernel_ask.query`
- `stop.answer`
- `escalate.message` if introduced later

Structured fields remain exact. Do not add variants for refs, cursors,
dimensions, timestamps, page/window/limit values, ingest payloads or write
payloads unless the accepted payload was wrong.

### v2

v2 added accepted narrative variants for `kernel_ask` and `stop`.

Result with `gpt-4o-mini`:

```text
report: ../rehydration-kernel-artifacts/operator/calibration-runs/2026-05-21T21-16-00Z-v2-full/report.json
total_cases: 25
match_count: 12
tool_match_count: 23
contract_valid_count: 21
shape_failed_count: 1
gate_passed: false
```

Diagnosis: the teacher still failed many structured arguments. v2 fixed
narrative ambiguity, but it did not give the teacher enough explicit structure
for exact KMP/MCP argument preservation.

### v3

v3 clarified structured goals and used `teacher_calibration_v2.md`.

Smoke result with `gpt-4o-mini`:

```text
report: ../rehydration-kernel-artifacts/operator/calibration-runs/2026-05-21T21-44-00Z-v3-smoke-final/report.json
total_cases: 3
match_count: 3
tool_match_count: 3
contract_valid_count: 3
shape_failed_count: 0
gate_passed: false
```

The smoke run is intentionally not a pass because `--limit 3` cannot satisfy
per-capability floors.

Best full `gpt-4o-mini` result observed:

```text
report: ../rehydration-kernel-artifacts/operator/calibration-runs/2026-05-21T22-28-00Z-v3-full-pass-attempt2/report.json
total_cases: 25
match_count: 24
tool_match_count: 25
contract_valid_count: 25
shape_failed_count: 0
overall_accuracy: 0.96
gate_passed: false
gate_failure_reason: kernel_ingest per_capability_accuracy 0.50 < floor 0.60
```

The remaining failure was `kernel_ingest`: the action was contract-valid but
misplaced `source` from `entry.metadata` into `coordinate.metadata`.

Full `gpt-4o` result:

```text
report: ../rehydration-kernel-artifacts/operator/calibration-runs/2026-05-21T22-49-00Z-v3-gpt4o-full/report.json
total_cases: 25
match_count: 22
tool_match_count: 25
contract_valid_count: 25
shape_failed_count: 0
overall_accuracy: 0.88
gate_passed: false
gate_failure_reason: kernel_ingest per_capability_accuracy 0.50 < floor 0.60
```

The `gpt-4o` failures were:

- two `kernel_ask` paraphrases that should be added as accepted narrative
  variants if v3 is kept;
- one `kernel_ingest` structured mismatch: `memory.relations[].evidence` was
  omitted even though the prepared payload declared it.

## Kernel Contract Check

The kernel API contains `MemoryRelation.evidence`:

```text
api/proto/underpass/rehydration/kernel/v1beta1/memory.proto
MemoryRelation.evidence = field 6
```

The MCP ingest mapper treats relation evidence as optional but requires
`why OR evidence` for non-structural relations:

```text
crates/rehydration-mcp/src/grpc/requests/ingest.rs
non-structural memory relations require why or evidence
```

Therefore the `gpt-4o` rich-ingest output is kernel-contract valid. It is still
not an accepted calibration action because this case is specifically measuring
whether the teacher preserves a prepared rich ingest payload exactly.

Do not mark this as a pass by adding an accepted structured variant that omits
`memory.relations[].evidence`. That would weaken the calibration target.

## Current Status

The calibration runner works.

The reporting works.

The dataset versioning flow works.

The teacher has not passed the gate yet. The blocker is exact preservation of
structured `kernel_ingest` payloads, especially metadata placement and optional
rich-relation fields declared in the prepared payload.

## Architectural Finding

The current calibration subject has:

```text
about
mode
task_family
goal
allowed_tools
visible_state
```

`visible_state` is intentionally narrow: known refs, known dimensions, active
cursor and budget. It cannot carry a typed prepared write or typed prepared
ingest payload.

That means the current ingest cases encode prepared payloads as narrative text
inside `goal`. This is weaker than the real architecture we want for Operator:
when a write is already prepared, Operator should receive structured prepared
arguments and decide whether to execute them, not recompile a long narrative
description into JSON.

## Decision Before v7.3

Do not start v7.3 corpus generation from this teacher yet.

Before v7.3, choose one of these paths:

1. Add a typed prepared-action/prepared-arguments field to the calibration
   subject and later trajectory subject, then recalibrate `kernel_ingest`.
2. Keep the current subject shape, but treat `kernel_ingest` calibration as
   measuring natural-language-to-payload compilation. This is not the desired
   Operator role and should be documented as such.

Recommended path: option 1.

After that change, add more `kernel_ingest` calibration cases. Two ingest cases
make the per-capability floor too brittle: one mismatch becomes 50%.

## Training Gate

No training should start from v7.2.5 until:

- full calibration passes the gate;
- `kernel_ingest` has enough cases to make the per-capability floor meaningful;
- structured write/ingest inputs match the intended Operator responsibility;
- the passing report path is recorded in this document and in the PR
  description.
