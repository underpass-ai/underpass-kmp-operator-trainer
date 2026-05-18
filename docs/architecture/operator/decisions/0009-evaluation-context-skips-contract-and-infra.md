# ADR 0009 — Evaluation context skips `*-contract` and `*-infra` in the first pass

Status: accepted (2026-05-18)
Companion to: [ADR 0008](0008-no-synthetic-contract-yet.md)

## Context

[01-bounded-contexts.md](../01-bounded-contexts.md) defines four crates
per context (`*-contract`, `*-domain`, `*-application`, `*-infra`). ADR
0008 already documented why the synthetic context skips `*-contract` —
no wire boundary today. For evaluation, both the contract and the infra
surface are absent in the first pass:

- **No wire types.** Today's use case is purely in-memory: it accepts a
  slice of `EvaluationPair` and returns a `EvaluationReport`. Nothing
  serialises.
- **No external adapters.** A future JSONL prediction reader and a
  report writer will need both `*-contract` (DTOs) and `*-infra`
  (adapters); they will land together when the first adapter has a
  concrete shape.

## Decision

The evaluation context ships with two crates only:

- `operator-evaluation-domain`
- `operator-evaluation-application`

The `*-contract` and `*-infra` crates are deferred until at least one
adapter requires them.

## Consequences

Positive:

- One fewer empty crate to maintain than ADR 0008's pattern would imply.
- The principle "create a crate only when the boundary it represents is
  real" is reinforced.

Negative:

- The "four crates per context" line in `01-bounded-contexts.md` is now
  contradicted by both synthetic (skips contract) and evaluation (skips
  contract + infra). The next bounded context that takes the same
  decision is reason to upgrade `01-bounded-contexts.md` itself from
  *must* to *should*, with an explicit rule for when a crate is
  required.

## When to revisit

Create `operator-evaluation-contract` + `operator-evaluation-infra` the
first time **any** of the following lands:

1. A use case consumes predictions from a file or network source.
2. A use case persists or publishes an `EvaluationReport`.
3. A consumer external to this repository (CLI, dashboard, paper
   pipeline) ingests evaluation outputs.

## Alternatives considered

- **Create empty `*-contract` + `*-infra` crates to preserve symmetry**
  — rejected for the same reasons as ADR 0008: empty crates rot, and
  the resulting pressure to fill them produces speculative DTOs.
- **Inline the future adapter in `operator-evaluation-application`** —
  rejected because that would push file-I/O dependencies into the
  application layer, which `no_io_runtime_outside_infra` forbids.
