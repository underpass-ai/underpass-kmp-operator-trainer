# ADR 0006 — Composition over inheritance, even simulated inheritance

Status: accepted (2026-05-18)

## Context

Rust does not have inheritance. It does have trait hierarchies and default
method bodies, which can be used to simulate inheritance. The legacy code
used a few such hierarchies (notably `SyntheticCaseTeacher` with default
method implementations).

## Decision

Operator avoids simulated inheritance:

- A type does not become "a kind of" another type via super-traits. If two
  types share behaviour, that behaviour is a separate trait that each type
  implements independently, or a value object that each type holds.
- Default method bodies are only allowed for derived views (read-only
  computations from other trait methods), never to hide state or to provide
  fallbacks.
- A type that needs to extend another's behaviour holds the other as a
  field (`Box<dyn Trait>` or generic) and delegates by composition.

## Consequences

- Type relationships are flat. Reading a type tells you everything it can
  do; there is no parent class to inspect.
- Two types that compose the same component can evolve independently.

## Examples in this codebase

- `CompositeActionContractValidator` holds a `Vec<Box<dyn
  Specification<...>>>` rather than being the root of a specification
  hierarchy. New specifications are added by composition.
- `AndSpec`/`OrSpec` are combinators that hold two specifications. They do
  not inherit from `Specification`; they implement it by delegation.
