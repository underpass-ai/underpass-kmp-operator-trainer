# ADR 0008 — No `operator-synthetic-contract` crate (yet)

Status: accepted (2026-05-18)

## Context

[01-bounded-contexts.md](../01-bounded-contexts.md) says every bounded
context is split into `*-contract`, `*-domain`, `*-application`,
`*-infra` (and optionally `*-cli`). The synthetic context produces
`TrainingTrajectory` values that already have a wire DTO in
`operator-shared-contract`. Nothing in synthetic itself crosses a
serialization boundary today: the generator returns domain types in
memory; the application use case returns a domain report; the in-memory
adapter is local.

A `SyntheticDatasetGenerationReport` could in principle be serialized to
disk (per-case metrics for offline analysis), but no consumer asks for
it today.

## Decision

`operator-synthetic-contract` is not created in this pass. The
contract surface for synthetic is the empty set; trajectories are
serialized via `operator-shared-contract::TrainingTrajectoryDto` and
nothing else leaves the process.

## Consequences

Positive:

- One fewer empty crate to maintain.
- The "four crates per context" rule from `01-bounded-contexts.md` is
  understood as a *should* — a context creates `*-contract` iff it owns
  a wire boundary.

Negative:

- The architectural symmetry is partially broken. New contributors must
  read this ADR to learn why synthetic skips it.

## When to revisit

Create `operator-synthetic-contract` the first time **any** of the
following lands:

1. A use case persists a `SyntheticDatasetGenerationReport` to disk or
   sends it over the wire.
2. The synthetic context needs a different wire representation of
   `TrainingTrajectory` than the shared one (unlikely).
3. An external system (CLI, dashboard) consumes synthetic-specific
   structured data.

## Alternatives considered

- **Create an empty `*-contract` crate to preserve symmetry** — rejected
  because empty crates rot, and the resulting code review pressure to
  "fill them" produces speculative DTOs.
- **Document this in `01-bounded-contexts.md` as a global rule** —
  deferred. We will collect at least two examples of "contract-less
  context" before generalising the rule.
