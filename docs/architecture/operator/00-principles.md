# Operator Architecture Principles

These rules are not aspirational. They are enforced by the
`operator-architecture-tests` crate. A pull request that violates a rule must
fix the rule or change this document first.

## 1. Hexagonal (ports and adapters)

Every bounded context is split into four crates: `*-contract`, `*-domain`,
`*-application`, `*-infra`. CLIs, when they exist, live in dedicated `*-cli`
crates.

- **Domain** never imports any other crate of the bounded context. It contains
  pure types and pure functions.
- **Application** depends on `domain` and on its own `ports.rs`. It does not
  import `infra`, `contract`, `serde_json`, `reqwest`, `tokio` or any
  third-party crate that talks to the outside world.
- **Contract** holds only serializable DTOs. It depends on `serde`,
  `serde_json` and nothing else.
- **Infra** depends on `domain`, `application` (to implement its ports) and
  `contract` (to map DTOs to and from domain). It is the only layer allowed to
  import `serde_json::Value`, `reqwest`, `tokio`, file system, time, random or
  network types.
- **CLI** crates are composition roots only. They parse arguments, wire
  application ports to infra adapters, run the use case and translate domain
  errors to process exit codes. They contain no business logic.

The "dependency arrow" always points inward:

```
cli ──▶ application ──▶ domain
 │            ▲
 ▼            │
infra ──▶ contract
```

`contract` is allowed to be a leaf depended on by both `application` (for
input/output schema, when a use case needs to expose a contract type at its
public API boundary) and `infra` (for mapping).

## 2. Pure DDD

- **Value objects** are immutable, validate in their named constructor, and
  define equality by value. They never expose mutable references to their
  internals.
- **Entities** have an identity. Identity is itself a value object (`StepId`,
  `TrainingTrajectoryId`, ...).
- **Aggregate roots** are the only types other contexts may reference across
  the boundary. They enforce invariants in their named constructors. Their
  state can only be mutated through methods that preserve invariants. Today
  `TrainingTrajectory` is the only aggregate.
- **Domain services** are stateless behaviours that act on aggregates and
  value objects. They live as traits in `domain` when their implementation can
  be substituted; otherwise as concrete `struct`s with `impl` methods.
- **Repositories** are abstract ports declared in `application/ports`. Their
  shape returns or accepts domain types only. Concrete implementations live in
  `infra`.
- **Specifications** capture domain rules that yield a boolean or a structured
  violation. They compose via `and`/`or` combinators. They live in `domain`.

There is no "anaemic model". A `TrainingTrajectory` is not a bag of public
fields with a constructor; it is an object that protects its invariants.

## 3. SOLID

- **S** — Single responsibility per type. A type that maps DTO to domain does
  not also validate cursors. A type that exports JSONL does not also build
  visible state.
- **O** — Open for extension via traits and composition. Closed for
  modification: adding a new tool to `KernelTool` must compile-fail every use
  case that pattern-matches without a wildcard, by design.
- **L** — Substitutability for ports. Two adapters that implement the same
  trait must behave identically with respect to the trait contract; in
  particular, error variants must be the same shape.
- **I** — Many small ports. A use case never depends on a "kitchen-sink"
  trait. If a use case only needs to read trajectories, it depends on
  `TrajectoryReader`, not on `TrajectoryRepository`.
- **D** — Use cases depend on traits, not on concrete adapters. The
  composition root wires the concrete adapter.

## 4. One file = one class

A file in `src/**/*.rs` contains exactly one of:

- one `struct` and its primary `impl` blocks,
- one `enum` and its primary `impl` blocks,
- one `trait` and its associated types,
- one free function group that forms a single domain rule (rare; preferred as
  a struct with associated functions).

`mod.rs` and `lib.rs` contain only module declarations and re-exports, plus
the crate-level dependency assertion test. Tests for a type live in the same
file as the type, behind `#[cfg(test)] mod tests`.

Exceptions are allowed only for tightly coupled wrapper newtypes (for example,
a generic `NonEmptyString` that backs many context-specific ID newtypes); the
exception must be explicit in the file's doc comment.

See [decisions/0002-one-file-one-class.md](decisions/0002-one-file-one-class.md).

## 5. Composition over inheritance

There is no inheritance in Rust. We still avoid trait inheritance trees that
emulate it. Prefer:

- holding a `Box<dyn SomeTrait>` field instead of being a sub-trait,
- composing specifications with `and`/`or` instead of nesting traits,
- delegating to a contained value object instead of "extending" it.

Default trait methods are allowed only to provide a derived view, never to
hide state.

## 6. No primitives across boundaries

A function signature in `domain` or `application` does not contain:

- `&str` when an enum or a named newtype exists for the value,
- `usize`/`u32`/`u64` when a named `Count`/`Limit`/`Window` exists,
- `serde_json::Value`,
- `Option<Option<T>>` or other nested-option shapes that hide intent.

Inside a function body, primitive types are fine. The boundary is what is
typed.

## 7. JSON is an infra concern

`serde_json::Value`, `json!(…)` and any handcrafted JSON construction live
exclusively in `*-infra` and `*-contract` crates. They never appear in
`*-domain` or `*-application` Cargo manifests, source files or tests.

Test fixtures that need to assert serialization behaviour live in `infra`
tests, not domain tests.

See [decisions/0004-no-serde-json-in-domain.md](decisions/0004-no-serde-json-in-domain.md).

## 8. No fallbacks, fail fast

A function that cannot perform its job returns an error. It does not return a
default value, an empty collection or a synthesized placeholder unless the
named return type explicitly carries that meaning (for example
`Option<Cursor>` when "no active cursor" is a legitimate domain state).

Constructors validate every invariant up front and refuse to build invalid
state.

## 9. No kernel dependency from Operator

No crate in this repository may declare a dependency on `rehydration-*` or any
other crate from the `rehydration-kernel` workspace. KMP and MCP are talked to
through their public protocol (gRPC/MCP JSON) at runtime, not through Rust
crate imports.

This rule is enforced by `operator-architecture-tests::no_kernel_deps`.

## 10. Architectural tests run on every commit

`operator-architecture-tests` scans Cargo manifests and source trees. The
table below shows which principles have an explicit test and which still
rely on code review. Untested principles are still binding; if you see one
of them violated in a PR, request a fix and consider adding a test.

| Principle | Test (under `tests/`) | Notes |
| --- | --- | --- |
| 1. Hexagonal | `no_io_runtime_outside_infra` | Bans `tokio`/`reqwest`/`tonic`/`tracing` in `*-domain`/`*-application`/`*-contract` manifests. |
| 1. Hexagonal (ports) | `ports_take_only_domain_types` | Bans `serde_json::Value`, `tool: &str`, `cursor_key: &str`, `json!` in `*-application/src/ports/*.rs`. |
| 2. Pure DDD | _convention_ | Reviewed at PR time. |
| 3. SOLID | _convention_ | Reviewed at PR time. |
| 4. One file = one class | `one_file_one_class` | One `pub struct`/`enum`/`trait` per file outside an allow-list. |
| 5. Composition over inheritance | _convention_ | Reviewed at PR time. |
| 6. No primitives across boundaries | `no_string_tool_or_cursor` | Bans `tool: &str` and `cursor_key: &str` in any operator source file. |
| 7. JSON is an infra concern | `no_serde_in_application_or_domain` + `no_serde_json_in_domain_or_application` | Manifests must not list `serde`/`serde_json` outside infra/contract; source files must not contain `serde_json::Value` or `json!(`. |
| 8. No fallbacks, fail fast | _convention_ | Constructors that return `T` and could fail are caught only by review and unit tests. |
| 9. No kernel dependency | `no_kernel_deps` | Manifests must not contain `rehydration-`. |
| 10. Architectural tests | `no_serde_json_value_in_application_or_domain_manifests` (workspace consistency) | Asserts every crate on disk is registered in the workspace and vice-versa. |
