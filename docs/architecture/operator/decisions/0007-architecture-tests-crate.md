# ADR 0007 — A dedicated `operator-architecture-tests` crate

Status: accepted (2026-05-18)

## Context

Architectural rules that are written only as documentation rot. The legacy
repo had a postmortem that listed the rules; some were enforced by
`#[cfg(test)] mod dependency_tests` inside each crate; many were not.

## Decision

A single test-only crate, `operator-architecture-tests`, owns the
enforcement of every rule listed in
[00-principles.md](../00-principles.md). It runs as part of
`cargo test --workspace`.

The crate reads:

- the workspace `Cargo.toml`,
- every member crate's `Cargo.toml`,
- every member crate's `src/` tree as text.

It never imports the production crates as Rust libraries; it inspects them
as files. This keeps the test crate fast and avoids cyclic dependencies.

The rules enforced today (one test file per rule under `tests/`):

1. **No kernel deps** (`no_kernel_deps`) — no operator manifest contains
   `rehydration-`.
2. **No serde_json in domain or application** (`no_serde_json_in_domain_or_application`)
   — `*-domain` and `*-application` manifests must not list `serde_json`;
   their source must not contain `json!(` or `serde_json::Value`.
3. **No serde in domain or application** (`no_serde_in_application_or_domain`)
   — same as #2, but for the plain `serde` crate (allowed in contract +
   infra only).
4. **No I/O runtime outside infra** (`no_io_runtime_outside_infra`) —
   `tokio`, `tokio-stream`, `reqwest`, `tracing`, `tracing-subscriber`,
   `tonic` and `prost` may not appear in `*-domain`, `*-application` or
   `*-contract` manifests.
5. **Ports take only domain types** (`ports_take_only_domain_types`) —
   files inside `*-application/src/ports/` must not contain
   `serde_json::Value`, `tool: &str`, `cursor_key: &str` or `json!(`.
6. **One file = one class** (`one_file_one_class`) — every `.rs` file
   declares at most one `pub struct`, `pub enum` or `pub trait`. Exceptions
   live in a `KNOWN_EXCEPTIONS` constant with a justification.
7. **No `tool: &str` / `cursor_key: &str` anywhere** (`no_string_tool_or_cursor`)
   — bans the substrings across every operator source file.
8. **Workspace consistency** (`no_serde_json_value_in_application_or_domain_manifests`)
   — every crate on disk is in the workspace `members`; every member is on
   disk. (The file name is historical; the test does the consistency
   sweep.)

The following principles are still convention-only and rely on PR review:

- **Pure DDD** (immutable value objects, aggregate boundary discipline).
- **SOLID** (single responsibility per type).
- **Composition over inheritance**.
- **No fallbacks, fail fast** (constructors that silently return defaults).
- **CLI thinness** — pending; the first CLI lives in a later bounded
  context.

## Consequences

- Rules cannot drift from documentation: if a rule changes, the test
  breaks.
- A new contributor can `cargo test -p operator-architecture-tests` and get
  a list of every violation, with line numbers.

## Alternatives considered

- **Per-crate inline tests** — partial, easy to forget when adding a new
  crate.
- **Pre-commit hooks** — bypassable; can drift between machines.
- **Clippy lints** — too rigid; cannot encode workspace-wide rules.
