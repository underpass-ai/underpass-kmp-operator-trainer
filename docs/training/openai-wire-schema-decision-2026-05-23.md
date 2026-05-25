# OpenAI Wire Schema Decision — Structured Output v2

## Context

During PR #46 v8.1.2 corpus generation, `stop:answer-ready` scenarios produced
`teacher_truncation` drops. A replay of
`scenario:stop:answer-ready:0010` returned HTTP 200 but finished with
`finish_reason=length` after 112.6s:

- request_id: `req_1f8d8e37f8cf4fd09fa16f616b09ffcc`
- request_bytes: `15830`
- subject_bytes: `1143`
- response_bytes: `13276`
- content_len: `8373`

The subject was small and valid. The failure was caused by the OpenAI
structured-output schema shape.

## v0 — Hybrid Flat Required Schema

The original schema emitted one flat action object with all fields required:

- `kind`
- `tool`
- `arguments`
- `reason`
- `answer`
- `evidence`
- `target_model`

This did not match the real `OperatorActionDto`, which is an externally tagged
enum. A simple `stop(answer_ready)` was forced to also produce tool-call-only
fields such as `tool` and `arguments`. Because `arguments` exposed every tool
argument shape, including the large `kernel_ingest` shape, the model could spend
thousands of tokens satisfying a field that does not belong to `stop`.

Result: systematic truncation risk for terminal actions.

## v1 — Discriminated Branch Schema

PR #39 tried a discriminated `anyOf`/`oneOf` schema. That fixed the structural
shape on paper but introduced behavioral bias: branches such as `escalate` were
structurally easier to satisfy than tool-call branches requiring copied refs and
arguments. Paid validation showed the model choosing the easier branch instead
of the correct action in regression scenarios.

Result: reverted. Do not reintroduce action-kind branch discrimination in the
provider schema without evidence that branch complexity is balanced.

## v2 — Flat Nullable Wire DTO + Mapper-Side Discrimination

The current fix keeps the OpenAI schema flat, strict and provider-compliant, but
uses nullable fields for non-applicable variant data:

- `kind` remains required and non-null.
- `tool`, `arguments`, `reason`, `answer`, and `target_model` are required but
  may be `null`.
- `tool`, `reason`, and `target_model` remain enum-constrained when non-null.
- `evidence` is always present and may be empty.

The adapter parses this into an OpenAI-specific wire DTO and maps it to the real
`OperatorActionDto`. Semantic consistency is enforced post-hoc:

- `tool_call` requires `tool` and `arguments`, and rejects `reason`, `answer`,
  `target_model`, and `evidence`.
- `stop` requires `reason`, rejects `tool`, `arguments`, and `target_model`, and
  allows `answer` plus `evidence`.
- `escalate` requires `reason` and `target_model`, and rejects `tool`,
  `arguments`, `answer`, and `evidence`.

If the model emits an inconsistent flat shape, the adapter returns a shape error
and the corpus row is dropped with a clear reason. The domain never sees an
invalid action.

## Decision

Variant discrimination lives in the OpenAI adapter mapper, not in the provider
schema and not in the domain.

The provider schema should be the least biased strict schema that OpenAI accepts.
The mapper is responsible for converting that wire shape into the real contract
DTO and rejecting inconsistent combinations.

## Observability

The adapter now emits request lifecycle events with:

- request size
- subject size
- elapsed time
- time to response headers
- response size
- status
- request id
- finish reason
- content length
- full `reqwest` transport error details

For `finish_reason=length`, drops persist a bounded `raw_content_tail` so a
future truncation can be diagnosed without repeating a paid call.

## Implementation Lesson — Primitive DTO Alignment

Applying the flat-nullable fix surfaced a second mismatch in `kernel_ingest`
metadata. The schema allowed metadata objects such as `{ "template": null }`,
but the shared DTOs use `BTreeMap<String, String>` for metadata values. The
mapper correctly rejected those rows.

This is the same bug class as the terminal-action truncation: the provider
schema was more permissive than the DTO boundary. The derived rule is:

- Variant applicability is enforced in the adapter mapper.
- Primitive value types must match DTO types exactly in the provider schema.
- Any schema change must be accompanied by a cross-check against DTO field
  optionality and primitive types.

The `kernel_ingest` replay used a fixture derived from a real scenario with
`prepared_action` removed. It is a schema test for the deployed model inference
path, not a corpus-generation reproduction; corpus generation bypasses the LLM
for `kernel_ingest` via the prepared-action fast-path.
