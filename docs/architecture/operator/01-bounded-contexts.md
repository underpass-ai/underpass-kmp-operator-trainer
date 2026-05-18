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
   ┌──────────┬────────────┼────────────┬──────────────┬─────────────────┐
   │          │            │            │              │                 │
synthetic evaluation    training      replay        runtime       benchmark-adapters
   │          │            │            │              │                 │
canonical  scoring,    manifests,    execute        compose         translate external
trajec-    coverage    readiness,    predicted      LLM, Op,        benchmark artifacts
tories     metrics     metrics       actions vs     KMP/MCP,        into Operator
                                     real MCP       budget,         trajectories
                                                    escalate
```

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
Rust dependency on `rehydration-*`**; it talks gRPC/MCP over the network.

## runtime

Owns the composition of an LLM, the Operator policy, the KMP/MCP client and
the budget/escalation policy at serving time.

## benchmark-adapters

Owns translation of external benchmark artifacts (LongMemEval, MemoryArena,
…) into Operator trajectories. It is the only context allowed to know about
benchmark schemas. It is forbidden to define Operator vocabulary.

## Allowed dependency edges

```
shared           ──▶  (nothing)
synthetic        ──▶  shared
evaluation       ──▶  shared
training         ──▶  shared, evaluation (only for readiness gates)
replay           ──▶  shared
runtime          ──▶  shared, evaluation (only for online contract checks)
benchmark-adapt. ──▶  shared
```

Edges not listed above are forbidden. In particular:

- `synthetic` does **not** depend on `evaluation` (generation must not know
  how it will be scored).
- `evaluation` does **not** depend on `synthetic` (scoring must work on any
  conformant trajectory regardless of how it was generated).
- `runtime` does **not** depend on `synthetic`, `training` or
  `benchmark-adapters`.
- No context depends on `rehydration-*` crates.
