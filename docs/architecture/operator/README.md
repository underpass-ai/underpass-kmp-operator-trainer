# Operator Architecture

This directory is the single source of truth for Operator's architecture. Every
public type, port, adapter and design decision is indexed here. If a piece of
the implementation is not documented in this tree, that is a bug.

## Index

### Cross-cutting

- [00-principles.md](00-principles.md) — Hexagonal, DDD, SOLID, naming and
  testing rules that every crate must follow.
- [01-bounded-contexts.md](01-bounded-contexts.md) — The six bounded contexts
  that make up Operator and their permitted dependencies.
- [02-design-patterns.md](02-design-patterns.md) — Catalog of design patterns
  used in this codebase (Specification, Mapper, Repository, Adapter, Factory,
  Builder, Strategy, Composite, Value Object, Aggregate Root) and where each
  one lives.
- [03-dependency-injection.md](03-dependency-injection.md) — How wiring is
  done. No service locator, no global state, no `lazy_static`. Composition
  roots are CLI crates.

### Bounded contexts

- [10-shared-context.md](10-shared-context.md) — The shared bounded context:
  vocabulary, action contract, trajectory model and validators every other
  context depends on. **First pass scope.**
- [20-synthetic-context.md](20-synthetic-context.md) — Canonical
  trajectory generation: capabilities, blueprints, generator port,
  in-memory adapter.
- [30-evaluation-context.md](30-evaluation-context.md) — Scoring and
  contract coverage: prediction pairs, outcomes, per-tool metrics.
- [40-replay-context.md](40-replay-context.md) — Execute predicted
  actions against MCP/KMP: predictions, outcomes, execution records,
  use case, in-memory stub adapter.
- [50-training-context.md](50-training-context.md) — Training run
  preparation: dataset provenance, readiness gates, manifests,
  `TrainingRun` aggregate root.
- 60-runtime-context.md *(pending)* — Compose LLM, Operator, KMP/MCP, budget.

External benchmark translation (LongMemEval, MemoryArena, …) is **not** an
Operator bounded context. Those adapters belong to the kernel
(`rehydration-kernel`) because their purpose is to measure KMP itself, not
to shape Operator's training surface.

### Shared bounded context — per-piece pages

- [shared/contract/README.md](shared/contract/README.md) — DTO index.
- [shared/domain/README.md](shared/domain/README.md) — Value object, entity and
  aggregate index.
- [shared/application/README.md](shared/application/README.md) — Use case and
  port index.
- [shared/infra/README.md](shared/infra/README.md) — Adapter and mapper index.

### Architecture Decision Records

- [decisions/0001-independent-repo.md](decisions/0001-independent-repo.md)
- [decisions/0002-one-file-one-class.md](decisions/0002-one-file-one-class.md)
- [decisions/0003-typed-tool-arguments.md](decisions/0003-typed-tool-arguments.md)
- [decisions/0004-no-serde-json-in-domain.md](decisions/0004-no-serde-json-in-domain.md)
- [decisions/0005-specification-pattern-validators.md](decisions/0005-specification-pattern-validators.md)
- [decisions/0006-composition-over-inheritance.md](decisions/0006-composition-over-inheritance.md)
- [decisions/0007-architecture-tests-crate.md](decisions/0007-architecture-tests-crate.md)
- [decisions/0008-no-synthetic-contract-yet.md](decisions/0008-no-synthetic-contract-yet.md)
- [decisions/0009-evaluation-context-skips-contract-and-infra.md](decisions/0009-evaluation-context-skips-contract-and-infra.md)
- [decisions/0010-replay-context-design.md](decisions/0010-replay-context-design.md) (§1 superseded by 0011)
- [decisions/0011-replay-context-talks-mcp-not-grpc.md](decisions/0011-replay-context-talks-mcp-not-grpc.md)
- [decisions/0012-training-context-design.md](decisions/0012-training-context-design.md)

## Scope so far

- **Pass 1** — `shared` bounded context (contract + domain + application +
  infra) plus `operator-architecture-tests` crate.
- **Pass 2** — `synthetic` bounded context skeleton: capability
  enumeration, blueprint, generation use case, in-memory adapter.
- **Pass 3** — `evaluation` bounded context: prediction pair, outcome,
  per-tool metric, report, use case wired to the shared-context
  `ActionContractValidator`.
- **Pass 4 (this PR + ADR 0010 + ADR 0011 + the outcomes-and-port PR
  before it)** — `replay` bounded context: predictions, outcomes,
  execution records, report, `KmpMcpClient` port, `ExecuteReplayUseCase`,
  in-memory stub adapter, end-to-end test covering every tool plus
  Stop/Escalate plus the failure-mode branch. Real MCP JSON-RPC client
  ships in a follow-up PR.

- **Pass 5** — `training` bounded context, full stack:
  - **5A** — domain skeleton (dataset provenance, readiness gates,
    training manifest, `TrainingRun` aggregate root).
  - **5B** — application (ports + use cases for read/write trajectory,
    evaluate readiness, assemble manifest, build/launch run).
  - **5C (this PR)** — infra (filesystem JSONL dataset writer, TOML
    manifest writer, `std::process::Command` trainer invoker, e2e
    integration tests).

The `runtime` context is still out of scope. Benchmark adapters are not
an Operator concern at all — they live in the kernel
(`rehydration-kernel`) because their purpose is to measure KMP itself.
