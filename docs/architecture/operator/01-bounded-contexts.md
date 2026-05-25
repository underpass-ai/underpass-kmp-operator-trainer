# Bounded Contexts

Operator is intentionally split into six bounded contexts. Each context owns
its own domain language, even when the words are spelled the same as in
another context. There are no shared "god" types across contexts beyond the
`shared` context.

```
                ┌────────────────────────┐
                │   shared (this pass)   │
                │   vocabulary           │
                │   action contract      │
                │   trajectory model     │
                └──────────▲─────────────┘
                           │
   ┌──────────┬────────────┼────────────┬──────────────┐
   │          │            │            │              │
synthetic evaluation    training      replay        runtime
   │          │            │            │              │
canonical  scoring,    manifests,    execute        compose
trajec-    coverage    readiness,    predicted      LLM, Op,
tories     metrics     metrics       actions vs     KMP/MCP,
                                     real MCP       budget,
                                                    escalate
```

Translation of external benchmark artifacts (LongMemEval, MemoryArena, …)
into Operator trajectories is **not** part of this repository — those
benchmark adapters belong to the kernel (`rehydration-kernel`), since their
purpose is to measure KMP performance against external workloads, not to
shape Operator's training surface. The kernel may emit
`TrainingTrajectory`-shaped artifacts through its own bounded contexts;
Operator consumes them through `TrajectorySource` adapters in
`training-infra`, never by importing benchmark schemas.

## shared

Owns the Operator vocabulary: `KernelTool`, `OperatorMode`, `Cursor`,
`OperatorAction`, `VisibleState`, `TrainingTrajectory`, the action contract
and the trajectory contract. All other contexts depend on `shared`. `shared`
depends on nothing else in Operator.

## synthetic

Owns canonical trajectory generation. It plans KMP/MCP use cases (one case
per declared capability) and produces a `TrainingTrajectory` stream. It does
not depend on benchmarks.

## evaluation

Owns prediction scoring, contract validation and capability coverage. It
consumes predictions and ground truths produced elsewhere; it does not
produce trajectories itself.

## training

Owns training run preparation: manifests, dataset provenance, readiness
gates, and metrics. It does not run a model; running is delegated to an
external trainer wired by a CLI in this context.

## replay

Owns executing a predicted `OperatorAction` against real MCP/KMP and
recording the observed outcome. It depends on `shared` and on a KMP/MCP
client adapter living in `replay-infra`. **The client adapter must not be a
Rust dependency on `rehydration-*`**; it talks MCP JSON-RPC over a process or
network boundary.

## runtime

Owns the composition of an LLM, the Operator policy, the KMP/MCP client and
the budget/escalation policy at serving time. The current implementation is
single-step only: predict, validate, optionally execute one MCP call, observe,
and persist the outcome.

## Allowed dependency edges

```
shared           ──▶  (nothing)
synthetic        ──▶  shared
evaluation       ──▶  shared
training         ──▶  shared, evaluation (only for readiness gates)
replay           ──▶  shared
runtime          ──▶  shared, replay infra (MCP DTOs/mappers),
                      synthetic DTOs/mappers (replay input parsing)
```

Edges not listed above are forbidden. In particular:

- `synthetic` does **not** depend on `evaluation` (generation must not know
  how it will be scored).
- `evaluation` does **not** depend on `synthetic` (scoring must work on any
  conformant trajectory regardless of how it was generated).
- `runtime` does **not** depend on `synthetic` or `training`.
- No context depends on `rehydration-*` crates.
