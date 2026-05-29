# Operator

Operator is a small specialist runtime that decides the next bounded KMP/MCP
action from visible memory state. It is **not** a general reasoning model, a
benchmark solver, or a replacement for KMP.

## Training approach (North Star)

The operator **only learns to use KMP**. It operates on **opaque, anonymized**
refs (`ref_0001`/`about_0001`), never on teacher domain topics — domain content
must never reach model-facing state. The model receives the MCP/tool schema
in-context and is SFT-trained to operate from the visible structural state.
A 2026-05-29 audit found v7/v8 training diverged by shipping un-anonymized domain
refs; see
[`docs/training/DIVERGENCE_AND_CORRECTIVE_PLAN_2026-05-29.md`](docs/training/DIVERGENCE_AND_CORRECTIVE_PLAN_2026-05-29.md).
Anonymization is now mandatory in the SFT prep pipeline.

This repository hosts the Operator product: typed domain, dataset model,
evaluation, replay, training manifests, and runtime composition. It is the
successor of the `underpass-operator-*` crates that lived inside
`rehydration-kernel/` and were stopped on 2026-05-18 (see postmortem there).

The current implementation status is **shared bounded context, in progress**.
Synthetic generation, evaluation, replay, training and benchmark adapters are
intentionally not part of this first pass.

## Repository shape

```
crates/
  operator-shared-contract      DTOs (serde) for the public Operator vocabulary
  operator-shared-domain        Value objects, entities, aggregates, domain errors
  operator-shared-application   Use cases and ports (hexagonal driving side)
  operator-shared-infra         Adapters and mappers (driven side)
  operator-architecture-tests   Test-only crate that enforces architectural rules

docs/architecture/operator/
  README.md                     Index of every architectural piece
  00-principles.md              Hexagonal, DDD, SOLID and naming rules
  01-bounded-contexts.md        The six bounded contexts and their boundaries
  10-shared-context.md          Detailed map of the shared bounded context
  shared/                       One markdown per public type / port / adapter
  decisions/                    ADRs (architecture decision records)
```

## How to build

```
cargo fmt
cargo build --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## License

Apache-2.0 (see `LICENSE`).

## Running E2E tests

Run `./scripts/e2e/regen.sh` before live E2E, replay validation, or infra-touching checks. It automates the version preflight in [docs/operations/preflight.md](docs/operations/preflight.md) and reports stale binaries, drifted Helm/Kubernetes state, missing certs, or endpoint/model mismatches before expensive tests run.

Example:

```bash
./scripts/e2e/regen.sh --verbose
```

Expected output uses `[OK]`, `[WARN]`, and `[FAIL]` lines and ends with an `N/M checks passed` summary.
