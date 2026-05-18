# Design Patterns Catalog

This document lists every design pattern used in Operator and where each one
lives. When a new pattern is introduced, add an entry here.

## Value Object

A small immutable type that represents a concept by value, validates in its
named constructor and exposes only read-only accessors.

- `operator-shared-domain::value_objects::NonEmptyString`
- `operator-shared-domain::ids::*` — `StepId`, `AboutId`,
  `TrainingTrajectoryId`, `TaskFamily`, …
- `operator-shared-domain::cursor::Cursor` and its sub-types
- `operator-shared-domain::budget::*`

## Aggregate Root

An entity that owns a consistency boundary and is the only door for external
code to mutate the aggregate. Its constructor refuses to build an invalid
aggregate.

- `operator-shared-domain::trajectory::TrainingTrajectory`

## Factory

A named constructor on a value object or aggregate that enforces all
invariants. There is no `pub` field constructor; types are built through
factories such as `TrainingTrajectory::new(...)`,
`AllowedTools::for_mode(...)`, `Cursor::around(...)`.

## Specification

A small named rule that yields either success or a `ContractViolation`.
Specifications compose with `and` / `or` to form a complete validator.

- `operator-shared-domain::specifications::ToolWithinModeSpec`
- `operator-shared-domain::specifications::ArgumentsReferenceKnownEntitiesSpec`
- `operator-shared-domain::specifications::CursorReachableFromVisibleSpec`
- `operator-shared-domain::specifications::BudgetAllowsActionSpec`

They are combined into `ActionContractValidator` in
`operator-shared-domain::contract::ActionContractValidator`.

See [decisions/0005-specification-pattern-validators.md](decisions/0005-specification-pattern-validators.md).

## Composite

`AndSpec` and `OrSpec` are composites over `Specification`. The final
`ActionContractValidator` is a composite that walks the list of
specifications and accumulates violations.

## Strategy

The application layer depends on `ActionContractValidator` as a trait. The
default strategy is the composite specification described above. Other
strategies (relaxed validators for debugging, strict validators for paper
runs) are infra-layer implementations of the same trait.

## Repository (Port)

`operator-shared-application::ports::TrajectoryReader` and
`TrajectoryWriter` are ports declared in `application`. They return domain
types. Concrete adapters live in `operator-shared-infra::adapters`.

## Mapper

A pure function (`struct` with associated functions) that translates a
contract DTO to a domain value, or vice versa, with explicit error variants.

- `operator-shared-infra::mappers::OperatorActionMapper`
- `operator-shared-infra::mappers::VisibleStateMapper`
- `operator-shared-infra::mappers::TrainingTrajectoryMapper`

## Adapter

Infrastructure type that implements an application port using an external
technology (file system, network, process).

- `operator-shared-infra::adapters::jsonl::JsonlTrajectoryReader`
- `operator-shared-infra::adapters::jsonl::JsonlTrajectoryWriter`

## Builder

Used only when a type needs many optional fields that depend on each other
non-trivially. Today we have:

- `operator-shared-domain::visible_state::VisibleStateBuilder` — assembles a
  `VisibleState` from progressive observations.

Builders are private to their owning module. They are not in the public API
of their crate.

## Composition Root

The `*-cli` crates own composition. They are the only place that constructs
concrete adapters and injects them into application use cases.

This context does not have CLIs in this pass; the equivalent is the
`operator-architecture-tests` crate, which constructs adapters for the sole
purpose of verifying their shape.
