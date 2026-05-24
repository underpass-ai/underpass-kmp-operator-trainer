# scenarios-v6 design — write-tool curriculum

## Purpose

scenarios-v6 scales the strict-contract corpus for v8.1.2. The v8.1
`action_correctness` report showed that Qwen2.5-0.5B learned tool selection and
most read-side arguments, but failed structured write arrays:

- `kernel_ingest.memory.entries[*]`
- `kernel_ingest.memory.relations[*]`
- `kernel_ingest.memory.evidence[*]`
- `kernel_ingest.memory.dimensions[*]`
- `kernel_write_memory.related[*]`

The v6 corpus therefore increases write-tool coverage and adds deterministic
curriculum variation inside prepared write payloads. It does not change the
strict contract, accepted actions, prompt v5, teacher, or drop gate.

## Corpus size

- scenarios-v5-1: 1,622 scenarios
- scenarios-v6 target: 2,144 scenarios
- Increase: about 32%

The generator now has 67 templates. `kernel_ingest` and
`kernel_write_memory` each have 10 templates, and the default scenario count
gives 32 variations per template. This yields about 320 scenarios per write
tool before train/eval splitting, enough for 100+ train rows per write tool
with margin.

## Subkinds covered

### `kernel_ingest`

Existing subkinds retained:

- `rich-relation`
- `anemic-fallback`
- `after-pre-read`
- `missing-provenance`
- `declared-dimensions`

New subkinds:

- `single-entry-minimal`
- `multi-entry-relations`
- `evidence-heavy`
- `dimension-heavy`
- `relation-sparse`

### `kernel_write_memory`

Existing subkinds retained:

- `prepared-payload`
- `smart-proof`
- `no-read-context`
- `related-refs`
- `minimal`

New subkinds:

- `related-none-short`
- `related-single-medium`
- `related-three`
- `related-five-long`
- `long-body`

## `kernel_ingest` curriculum

Each ingest template cycles through 10 deterministic variation profiles:

| Profile | entries | relations | evidence | dimensions | provenance |
| --- | ---: | ---: | ---: | ---: | --- |
| 0 | 1 | 0 | 0 | 0 | minimal |
| 1 | 1 | 0 | 1 | 0 | full |
| 2 | 1 | 1 | 1 | 2 | minimal |
| 3 | 2 | 1 | 3 | 2 | full |
| 4 | 2 | 3 | 0 | 2 | minimal |
| 5 | 5 | 0 | 3 | 5 | full |
| 6 | 5 | 3 | 5 | 5 | full |
| 7 | 2 | 3 | 5 | 5 | minimal |
| 8 | 5 | 1 | 5 | 2 | full |
| 9 | 5 | 3 | 1 | 0 | minimal |

Notes:

- `entries=0` is not generated. The current `kernel_ingest` contract requires
  at least one entry, so an empty-entry payload would be invalid corpus data.
- `dimensions=0` is generated. Entries still carry coordinates, but the
  payload does not declare dimensions in that profile. This covers the
  incremental append case where dimension declarations are already known.
- Minimal provenance means the currently required contract fields:
  `source_kind`, `source_agent`, and `observed_at`.
- Full provenance adds `correlation_id` and `causation_id`.

## `kernel_write_memory` curriculum

The current `kernel_write_memory` contract contains only `summary`, `body`, and
optional `related`. It does not yet model `evidence[]`, memory kind, or typed
relation metadata. v6 therefore varies only the fields that exist today:

| Profile | related refs | body length |
| --- | ---: | --- |
| 0 | 0 | short |
| 1 | 1 | short |
| 2 | 1 | medium |
| 3 | 3 | medium |
| 4 | 5 | long |
| 5 | 0 | medium |
| 6 | 3 | long |
| 7 | 5 | medium |

Future full relation typing for write-memory is out of scope for v8.1.2. If
the domain adds those fields later, they should get their own curriculum and
field-correctness rules.

## Why prepared actions

All new write-tool scenarios use `subject.prepared_action`. This is deliberate:
v7.3 established that write actions are strict-contract execution tasks, not
teacher policy-choice tasks. The teacher should not reconstruct long memory
payloads from prose. The corpus row should preserve the payload the contract
requires, and the trained model should learn to copy it exactly.

