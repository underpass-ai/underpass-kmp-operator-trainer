# Operator v7.3 regression pack baseline — 2026-05-23

## Run

Run id: `regression-pack-v7-20260523T060858Z`

Command:

```bash
cargo run --release -p operator-synthetic-cli --bin operator-regression-pack-v7 -- \
  --scenarios ../rehydration-kernel-artifacts/operator/scenarios-v4/scenarios.jsonl \
  --pack docs/training/regression_pack_v7.txt \
  --output ../rehydration-kernel-artifacts/operator/regression-pack-v7-20260523T060858Z \
  --api-base https://api.openai.com/v1 \
  --api-key-file /tmp/openai.txt \
  --prompt crates/operator-synthetic-infra/prompts/teacher_calibration_v5.md \
  --model gpt-4o-mini \
  --temperature 0.0
```

Inputs:

| Input | SHA-256 |
|---|---|
| `../rehydration-kernel-artifacts/operator/scenarios-v4/scenarios.jsonl` | `006521f673df2ea8927b4cf6b15c32d904c1104e5ecad912ab3c63467684bf6b` |
| `crates/operator-synthetic-infra/prompts/teacher_calibration_v5.md` | `87e26adf71049c165daa68ea016091846f576b9d4902de5276ce37e81956913c` |
| `docs/training/regression_pack_v7.txt` | `fd8c491521e05692828018b3a756381dfa53bfb130cc22e9bc8a7d0ee864e2ba` |

Output:

```text
accepted_count: 0
dropped_count: 3
gate_passed: false
```

`trajectories.jsonl` has 0 rows. `dropped.jsonl` has 3 rows.

## Baseline rows

### `scenario:kernel_inspect:after-near:0007`

Drop:

```text
reason: target_mismatch
message: expected inspect, got kernel_goto
teacher_finish_reason: stop
subject_hash: d269a86d55db2d2021eab64ee6456ec878546f59e0c0e0dfc8d387721d7b92ac
```

Predicted action:

```json
{
  "kind": "tool_call",
  "tool": "kernel_goto",
  "arguments": {
    "cursor": {
      "kind": "ref",
      "target": "about:incident:checkout-errors:kernel_inspect:after-near:case-000:node:hypothesis:000"
    }
  }
}
```

Disposition: matches the diagnosed failure pattern from the failed smoke.

### `scenario:stop:no-candidate:0028`

Drop:

```text
reason: target_mismatch
message: expected stop, got kernel_ask
teacher_finish_reason: stop
subject_hash: 6bd97f8faba0e1d901053b605583be11a7d603e15e4f057cddcea3dcdfef40e1
```

Predicted action:

```json
{
  "kind": "tool_call",
  "tool": "kernel_ask",
  "arguments": {
    "query": "What is the current status of the iOS login loop issue?"
  }
}
```

Disposition: matches the diagnosed adversarial policy preference (`kernel_ask`
over `stop(no_candidate)`).

### `scenario:kernel_goto:temporal-cursor:0021`

Drop:

```text
reason: semantic_mismatch
message: semantic mismatch at goto.cursor.kind: expected temporal, got ref
teacher_finish_reason: stop
subject_hash: f5c19cf4cfbdbf4cc7cdc8bc4a90c3d74ed8ee1c365877c1038e98cca23c4fdb
```

Predicted action:

```json
{
  "kind": "tool_call",
  "tool": "kernel_goto",
  "arguments": {
    "cursor": {
      "kind": "ref",
      "target": "about:migration:legacy-cache-removal:kernel_goto:temporal-cursor:case-000:node:state:000"
    }
  }
}
```

Disposition: validates the new semantic acceptance gate. The coarse target
match passed (`kernel_goto`), and the semantic gate correctly rejected
`cursor.kind=ref` where the scenario expected `temporal`.

## Decision

The regression pack reproduced the three diagnosed failures without a new
failure mode:

- `kernel_inspect:after-near` still predicts `kernel_goto`;
- `stop:no-candidate` still predicts `kernel_ask`;
- `kernel_goto:temporal-cursor` still predicts a `ref` cursor and is now
  captured as `semantic_mismatch`.

The baseline is sealed. PR #38 can move from draft to ready-for-review.
