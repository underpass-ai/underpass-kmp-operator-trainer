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
- 30-evaluation-context.md *(pending)* — Scoring and contract coverage.
- 40-replay-context.md *(pending)* — Execute predicted actions against MCP/KMP.
- 50-training-context.md *(pending)* — Run manifests, readiness, metrics.
- 60-runtime-context.md *(pending)* — Compose LLM, Operator, KMP/MCP, budget.
- 70-benchmark-adapters-context.md *(pending)* — External benchmark translation.

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

## Scope so far

- **Pass 1** — `shared` bounded context (contract + domain + application +
  infra) plus `operator-architecture-tests` crate.
- **Pass 2 (this PR)** — `synthetic` bounded context skeleton: capability
  enumeration, blueprint, generation use case, in-memory adapter,
  end-to-end test wiring all of the above.

Evaluation, replay, training, runtime and benchmark adapters are out of
scope until synthetic is exercised by at least one real consumer.
