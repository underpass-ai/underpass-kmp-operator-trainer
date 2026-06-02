# ADR 0005 — Action contract validation uses the Specification pattern

Status: accepted (2026-05-18)

## Context

`underpass-operator-shared-domain::action_contract` was a single 2,159-line
file with ~50 validation functions. Each rule was a `fn` and the call sites
strung them together with `?`. Adding or relaxing a rule required scanning
the whole file.

## Decision

Each contract rule is a `Specification` — a small named struct with a
single behaviour method that returns either success or a
`ContractViolation`:

```rust
pub trait Specification<T> {
    fn evaluate(&self, value: &T) -> Result<(), ContractViolation>;
}
```

Specifications take no constructor arguments — the `ActionContractSubject`
they evaluate carries the mode, visible state and budget. They compose with
the `AndSpec` / `OrSpec` combinators, or as a `Vec<Box<dyn Specification<...>>>`:

```rust
let spec = AndSpec::new(
    Box::new(ToolWithinModeSpec::new()),
    Box::new(AndSpec::new(
        Box::new(ArgumentsReferenceKnownEntitiesSpec::new()),
        Box::new(AndSpec::new(
            Box::new(CursorReachableFromVisibleSpec::new()),
            Box::new(BudgetAllowsActionSpec::new()),
        )),
    )),
);
```

The `ActionContractValidator` is the trait that wraps a specification
collection and produces a typed report:

```rust
pub trait ActionContractValidator {
    fn validate(
        &self,
        action: &OperatorAction,
        mode: OperatorMode,
        visible: &VisibleState,
    ) -> Result<(), ContractViolations>;
}
```

The default implementation `CompositeActionContractValidator` walks a
`Vec<Box<dyn Specification<...>>>` and accumulates violations rather than
failing at the first one. This gives the dataset builder a complete view of
what is wrong with a candidate trajectory.

## Consequences

- Each rule is its own file and its own test.
- Rules are reusable across contexts (synthetic, evaluation, replay all
  share the same `ArgumentsReferenceKnownEntitiesSpec`).
- Adding a rule is "create one file, add it to the composite". Removing or
  relaxing a rule is "delete one file, remove from composite".

## Alternatives considered

- **Free functions** — the legacy choice. Rejected: no composability, no
  per-rule tests, no per-rule documentation.
- **Builder with method chaining** — rejected because rule selection is a
  domain decision, not a fluent API affordance.
