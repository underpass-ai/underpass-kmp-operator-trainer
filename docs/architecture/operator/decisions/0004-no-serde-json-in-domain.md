# ADR 0004 — `serde_json` is forbidden in domain and application

Status: accepted (2026-05-18)

## Context

The legacy `underpass-operator-shared-domain::action_contract` (2,159 lines)
took `serde_json::Value` arguments and validated them with hand-written JSON
inspectors. Domain logic relied on JSON shape.

## Decision

`serde_json` is forbidden in every `*-domain` and `*-application` crate. It is
permitted everywhere else, including:

- `operator-shared-contract` (DTO definitions),
- every `*-infra` crate (mappers, adapters and JSONL I/O),
- every `*-cli` composition-root crate (translating wire payloads),
- `operator-architecture-tests` (introspection only).

Domain code accepts and returns domain types. Application code accepts and
returns domain types. If a use case must accept "JSON payload" data from
outside the process, the CLI/composition root translates it via an infra
mapper before invoking the use case.

`json!(…)` is forbidden everywhere except in infra tests that explicitly
verify serialization behaviour.

## Consequences

- Domain validators express their rules in terms of value objects, not
  field-shape patterns.
- Mappers gain a clearer responsibility: they are the only place that
  decodes wire format.
- Application use cases are testable without touching JSON.

## Enforcement

`operator-architecture-tests::no_serde_json_in_domain_or_application` opens
every `*-domain` and `*-application` Cargo manifest and fails if `serde_json`
is listed. It also greps source files for `json!` and
`serde_json::Value` in those crates and fails on a hit.
