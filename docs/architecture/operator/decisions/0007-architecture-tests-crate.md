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

The rules enforced:

1. **No kernel deps** — no operator manifest contains `rehydration-`.
2. **No serde_json outside contract/infra** — `*-domain` and
   `*-application` manifests must not list `serde_json` as a dependency, and
   their source must not contain `json!` or `serde_json::Value`.
3. **One file = one class** — every `.rs` file in `src/` declares at most
   one `pub struct`, one `pub enum`, or one `pub trait`. Exceptions are
   listed in a constant in the test crate with a justification comment.
4. **No `tool: &str` / `cursor_key: &str`** — these substrings must not
   appear in any operator source file (outside an explicit allowlist of
   infra mapper files).
5. **CLI thinness** — every `*-cli` crate's `main.rs` is under 200 lines and
   imports at least one `*-application` use case. (Pending; first CLI lives
   in a later pass.)
6. **Workspace member list** — every crate inside `crates/` is registered
   in the workspace `members` list, and every member is a real directory.

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
