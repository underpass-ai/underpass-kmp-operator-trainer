# Teacher Calibration Runbook

This runbook covers v7.2.5: measuring a teacher model against human-authored
Operator calibration cases before using it to generate a realistic corpus.

## Purpose

The calibration suite answers one question:

```text
Does the teacher choose the correct next KMP/MCP operator action?
```

It does not train a model, generate SFT rows, replay a full corpus, or score
final answers.

## Artifacts

Calibration cases are runtime artifacts outside the repo:

```text
../rehydration-kernel-artifacts/operator/calibration-cases-v1/cases.jsonl
```

Reports are written outside the repo:

```text
../rehydration-kernel-artifacts/operator/calibration-runs/<run-id>/report.json
```

The committed prompt is:

```text
crates/operator-synthetic-infra/prompts/teacher_calibration_v1.md
```

## Case Shape

Each JSONL row is one `CalibrationCaseDto`:

```json
{
  "case_id": "calib:technical_incident:wake-current-about",
  "domain_theme": "technical_incident",
  "category": "happy",
  "subject": {
    "about": "incident:payments-timeout",
    "mode": "read",
    "task_family": "calibration.kernel_wake.technical_incident.current_about",
    "goal": "Load the current incident memory before selecting evidence.",
    "allowed_tools": ["kernel_wake", "kernel_ask"],
    "visible_state": {
      "known_refs": [],
      "known_dimensions": [],
      "budget": { "calls_remaining": 4, "tokens_remaining": 1800 }
    }
  },
  "accepted_actions": [
    {
      "kind": "tool_call",
      "tool": "kernel_wake",
      "arguments": { "about": "incident:payments-timeout" }
    }
  ],
  "expected_action_rationale": "Human-facing explanation for review and failure reports."
}
```

The teacher sees only `subject`. It never sees `accepted_actions` or
`expected_action_rationale`.

## Adding A Case

1. Pick one of the five domain themes:
   `technical_incident`, `software_migration`, `bug_investigation`,
   `product_planning`, `smart_writing_session`.
2. Pick category: `happy` or `adversarial`.
3. Make `allowed_tools` exactly match `mode`.
4. Put every referenced memory ref, dimension or temporal cursor in
   `visible_state`.
5. Add 1-3 `accepted_actions`. Multiple accepted actions are allowed only for
   genuine ambiguity and must belong to the same capability.
6. Write `expected_action_rationale` for humans. Do not rely on it as model
   input.

For prepared writes or ingests, the `goal` must contain the prepared data the
teacher needs to build the action. Do not hide required write fields in the
accepted action only.

## Smoke Run

Use a small limit before any paid run:

```bash
cargo run --release -p operator-synthetic-cli --bin operator-teacher-calibration -- \
  --cases ../rehydration-kernel-artifacts/operator/calibration-cases-v1/cases.jsonl \
  --prompt crates/operator-synthetic-infra/prompts/teacher_calibration_v1.md \
  --api-base https://api.openai.com/v1 \
  --api-key-file /tmp/openai.txt \
  --model gpt-4o-mini \
  --temperature 0 \
  --output ../rehydration-kernel-artifacts/operator/calibration-runs/2026-05-21T15-30-00Z-smoke \
  --limit 3
```

`--limit 0` means no limit.

## Full Run

```bash
cargo run --release -p operator-synthetic-cli --bin operator-teacher-calibration -- \
  --cases ../rehydration-kernel-artifacts/operator/calibration-cases-v1/cases.jsonl \
  --prompt crates/operator-synthetic-infra/prompts/teacher_calibration_v1.md \
  --api-base https://api.openai.com/v1 \
  --api-key-file /tmp/openai.txt \
  --model gpt-4o-mini \
  --temperature 0 \
  --output ../rehydration-kernel-artifacts/operator/calibration-runs/2026-05-21T15-30-00Z \
  --limit 0
```

The CLI exits `0` only when gates pass. It exits non-zero for failed gates,
shape failures that push the score below threshold, source errors, prompt/key
precheck errors, or provider failures.

## Report

`report.json` contains:

- dataset path and sha256;
- prompt path and sha256;
- model, API base, temperature and timestamps;
- `match_count`;
- `tool_match_count`;
- `contract_valid_count`;
- `shape_failed_count`;
- `overall_accuracy`;
- `per_capability_accuracy`;
- `per_category_accuracy`;
- `case_results` with rationale only on failures.

Primary gate:

```text
overall_accuracy >= 0.80
per_capability_accuracy >= 0.60 for every capability
```

Per-category metrics are diagnostic. If adversarial accuracy is weak, treat the
teacher as unsafe for v7.3 even if the current gate passes.

## Failure Handling

Do not patch the cases to fit the teacher.

If a capability fails:

1. Inspect failing `case_results`.
2. Confirm the case is honest and has enough subject information.
3. If the case is bad, create `calibration-cases-v2` and document the fix.
4. If the case is good, adjust `teacher_calibration_vN.md`.
5. Rerun smoke, then full calibration.

If the teacher output does not parse as `OperatorActionDto`, it counts as
`shape_failed_count`; the runner does not repair JSON or strip Markdown.

## Versioning

Increment the dataset version when cases change:

```text
calibration-cases-v1/
calibration-cases-v2/
```

Increment the prompt version when policy or examples change:

```text
teacher_calibration_v1.md
teacher_calibration_v2.md
```

Every report records both sha256 values so calibration results remain
auditable.
