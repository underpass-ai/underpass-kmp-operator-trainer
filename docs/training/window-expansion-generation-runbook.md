# Window-Expansion Generation Runbook

This runbook covers generating window-expansion SFT trajectories: driving the
multi-step Operator loop with a teacher that demonstrates how to widen a KMP
temporal window (`kernel_rewind` / `kernel_forward`) across several calls until
a period's evidence is covered, then emitting the accepted trajectories as an
SFT dataset.

## Purpose

The generator answers one question per episode:

```text
Can the teacher cover the whole period by widening the window, and what does
that demonstration look like as a (visible_state -> action) SFT example?
```

It does NOT compute the period's count or score final answers — that is the
downstream reader's job. The Operator's job is coverage: retrieve the period's
entries. The generator runs the real loop, verifies coverage, and writes the
accepted trajectories. Episodes that stop short of coverage or surface a
conflict are dropped with an auditable reason.

## Binary

```text
operator-generate-window-expansions   (crate: operator-runtime-cli)
```

## Episode shape

Each input JSONL row is one `WindowExpansionEpisodeDto`. See
`window-expansion-episodes.example.jsonl` for runnable examples.

```json
{
  "about": "about:incident-payments",
  "goal": "Count the paid workshops in the last four months. Open the window with kernel_rewind toward the period start and kernel_forward toward the end, widening until coverage is complete, then stop.",
  "initial_window": 6,
  "max_iterations": 8,
  "token_budget": 8192,
  "expected_refs": ["about:incident-payments:node:deploy-jan"]
}
```

| Field | Required | Meaning |
| --- | --- | --- |
| `about` | yes | The memory the session operates over. Must exist in the target kernel. |
| `goal` | yes | The question + the "widen until covered" instruction the policy is given. Conveys the period. |
| `initial_window` | yes | Seed `window` size for the first temporal move (positive). |
| `max_iterations` | yes | Max expansion calls (positive). The call budget is `max_iterations + 1` (one reserved for the terminal stop/escalate). |
| `token_budget` | no (default 8192) | Total token budget for the session. |
| `expected_refs` | no | Gold coverage set: the period's entry refs the window must retrieve. When present this is the **authoritative** accept criterion; absent, coverage is judged from the kernel's in-band signals. |

`about` and any `expected_refs` must reference **real entries in the target
kernel** — the generator runs the live loop, so illustrative ids will not
retrieve anything.

## Accept / drop

Per episode the generator compiles a read session, runs the loop (the teacher
demonstrates expansion), then:

- **Accept** (expand to trajectories) when coverage is reached and no conflict
  surfaced. With `expected_refs`, "covered" means every gold ref was retrieved;
  without them, the kernel's coverage signal (`Complete` or `saturated`) decides.
- **Drop** otherwise, with a reason:
  - `conflict_blocking` — a contradiction surfaced (would force an escalate).
  - `missing_gold_refs` — the gold period set was not fully retrieved.
  - `incomplete` / `saturated`'s absence — the signal oracle judged the period
    not covered.

A `--max-drop-rate` gate fails the run if too many episodes drop.

## Invocation

The teacher and KMP endpoints are flags, so the same binary runs against a live
in-cluster kernel or a local/replay endpoint.

```bash
operator-generate-window-expansions \
  --episodes-jsonl ../rehydration-kernel-artifacts/operator/window-expansion/episodes.jsonl \
  --output-dir     ../rehydration-kernel-artifacts/operator/window-expansion/runs/<run-id> \
  --task-family    runtime.window_expansion \
  --max-drop-rate  0.4 \
  --teacher-api-base <openai-compatible base, e.g. the vLLM mTLS endpoint> \
  --teacher-model    <a capable model served there> \
  --teacher-prompt   crates/operator-synthetic-infra/prompts/teacher_calibration_v6.md \
  --kmp-mcp-endpoint <kernel gRPC endpoint> \
  --kmp-mcp-transport stdio \
  --kmp-mcp-stdio-command rehydration-mcp
```

The teacher API key may also come from `OPERATOR_TEACHER_API_KEY`. Do not point
the teacher at OpenAI; use the vLLM mTLS endpoint or an Anthropic-compatible
gateway.

### KMP connectivity

`KmpMcpHttpExecutor` carries no mTLS. The Operator reaches the kernel through the
MCP layer: the default `stdio` transport spawns `rehydration-mcp`, which does the
kernel gRPC. Run in-cluster (a k8s Job) so it reaches the kernel service
directly; the mTLS ingress (`rehydration-kernel.underpassai.com`) is for external
clients. Everything runs as a rootless k8s Job — no host binaries.

## Output

```text
<output-dir>/window_expansions.sft.jsonl    # accepted trajectories, {prompt, completion} per ADR 0012
```

The run prints `episodes / accepted / dropped / drop_rate / trajectories` plus a
per-drop `about` + `reason` line to stderr.

## Prerequisites and sequencing

1. The kernel must serve the in-band coverage signals (`coverage.dimensions`,
   `proof.frontier_size`, `quality`) — landed in kernel `feat/kmp-context-coverage-signals`.
2. The teacher prompt is `teacher_calibration_v6.md` (window-expansion aware).
3. The Operator serve/train prompt is `operator_system_prompt_full_v2.txt`. It
   is staged: the served policy still embeds v1 so a model that was not trained
   on the window-expansion section is not handed it. Flip the `include_str!` to
   v2 only together with training the window-expansion model (Phase C), then
   revalidate.
4. Generation is a teacher-LLM run; size `--max-iterations` and the episode
   count against your budget.
