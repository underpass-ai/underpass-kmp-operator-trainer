# Dependency Injection

Operator uses **constructor injection** only. There is no service locator, no
global registry, no `lazy_static`, no `once_cell::sync::Lazy`, no
thread-local "current" anything.

## How a use case is wired

```rust
// in application crate
pub struct ValidateTrajectoryUseCase<V: ActionContractValidator, R: TrajectoryReader> {
    validator: V,
    reader: R,
}

impl<V: ActionContractValidator, R: TrajectoryReader> ValidateTrajectoryUseCase<V, R> {
    pub fn new(validator: V, reader: R) -> Self {
        Self { validator, reader }
    }

    pub fn execute(&self, input: ValidateTrajectoryInput) -> Result<ValidateTrajectoryOutput, ValidateTrajectoryError> {
        // pure orchestration on domain types
    }
}
```

```rust
// in cli crate (composition root)
fn main() -> std::process::ExitCode {
    let validator = CompositeActionContractValidator::default_for_shared_context();
    let reader = JsonlTrajectoryReader::open(path)?;
    let use_case = ValidateTrajectoryUseCase::new(validator, reader);
    match use_case.execute(input) {
        Ok(_) => std::process::ExitCode::SUCCESS,
        Err(e) => { eprintln!("{e}"); std::process::ExitCode::FAILURE }
    }
}
```

## Generic vs `dyn`

Both forms are acceptable:

- **Generic parameters** when there is exactly one production implementation
  and the use case is performance-sensitive or invoked many times. Cheaper
  monomorphisation, no vtable.
- **`Box<dyn Trait>`** when a use case is constructed once per process and
  the surface needs to support many implementations at runtime (for example,
  multiple validator strategies selected by CLI flag).

Choose one explicitly per use case. Do not switch back and forth.

## Constructors are explicit

A constructor is `pub fn new(...)` (or a domain-flavoured factory name like
`for_mode`, `from_jsonl_path`, …). `Default` is implemented only when a
truly-default value exists (for example `BudgetSnapshot::unbounded()` is
explicit, not derived).

## Cyclic dependencies

Two use cases that need each other are a smell. Resolve by extracting the
shared concept into `domain` or by introducing an application port that one
use case depends on and the other implements.

## Lifetimes

Use cases take owned services in their constructor. They do not hold
references with non-`'static` lifetimes. Adapters that wrap a connection
internally manage their own lifetime.
