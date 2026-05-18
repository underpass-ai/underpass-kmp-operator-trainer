# ADR 0001 — Operator lives in its own repository

Status: accepted (2026-05-18)
Supersedes: in-kernel `underpass-operator-*` crates inside `rehydration-kernel`

## Context

The previous Operator implementation lived as 21 crates inside
`rehydration-kernel/crates/underpass-operator-*`. The postmortem
(`rehydration-kernel/docs/product/operator-architecture-postmortem-2026-05-18.md`)
documented that this implementation accumulated benchmark-shaped logic,
1,000+ line files with mixed responsibilities, primitive obsession, JSON
leaks into domain, and a forbidden direct Rust dependency from
`underpass-operator-replay-cli` on `rehydration-mcp`.

Architecture tests inside the same workspace can in principle enforce a
boundary, but in practice the proximity of `rehydration-*` crates pulls
contributors toward easy imports.

## Decision

Operator is a separate repository at `/home/tirso/ai/developents/operator`
with its own Cargo workspace, its own `rust-toolchain.toml`, its own CI and
its own version stream. It talks to KMP/MCP over their public protocols
(gRPC/MCP JSON) at runtime, never through Rust crate imports.

## Consequences

Positive:

- The "no rehydration-* dependency" rule is enforced by repo boundary, not
  by a test that contributors can silence locally.
- Operator can be published independently (HuggingFace model, dataset,
  small runtime).
- Operator CI is small and fast.
- Operator history is not contaminated by the previous attempt.

Negative:

- KMP/MCP contract changes must be coordinated. The Operator repo will pin a
  versioned KMP gRPC proto and MCP JSON schema, fetched from a published
  artifact or vendored. Until that is set up, replay-infra is allowed to
  hardcode the minimal proto subset Operator needs.
- Cross-cutting refactors that touch KMP and Operator together cost two
  PRs.

## Alternatives considered

- **Same workspace, new clean crates next to the legacy ones** — rejected
  because architectural rules are easier to break than to reinforce, and the
  legacy crates would keep tempting contributors to reuse fixtures.
- **Same workspace, rewrite in place** — rejected because git history would
  remain contaminated and the architecture tests would be amber during the
  migration.
