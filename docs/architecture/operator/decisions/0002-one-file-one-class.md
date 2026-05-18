# ADR 0002 — One file = one class

Status: accepted (2026-05-18)

## Context

The legacy Operator crates had files with 13+ public types in a single
`.rs` file and several files over 1,000 lines combining DTO mapping,
validation, fixture construction and policy. Type-level navigation became
impossible. Refactors required scanning entire files for unrelated
responsibilities.

## Decision

A source file in any Operator crate contains exactly one of:

- one `pub struct` and its `impl` blocks,
- one `pub enum` and its `impl` blocks,
- one `pub trait` and its associated types,
- one group of associated functions that forms a single domain rule
  (rare; preferred as a struct).

`mod.rs` and `lib.rs` may only contain `mod`/`pub use` declarations and the
self-check dependency test.

Tests for a type live in the same file behind `#[cfg(test)] mod tests`.

Private helpers used by a single type may live in the same file when they
do not constitute a separate concept. If they have their own name and shape
(for example a builder for a value object), they go to their own file.

## Consequences

Positive:

- File name = type name = grep target = doc target.
- Pull request reviews focus on a single concept per file.
- `git blame` is meaningful again.

Negative:

- More files. We accept this in exchange for navigability.
- Module declarations are longer. We accept this and require `mod.rs` to
  stay tiny.

## Enforcement

`operator-architecture-tests::one_file_one_class` scans the source tree and
fails when a file declares more than one `pub struct|enum|trait` outside an
explicit allowlist (the allowlist is a constant in the test crate, and
entries must justify the exception with a comment).
