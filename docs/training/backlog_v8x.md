# Backlog — v8.x corpus and teacher investigations

## Context

v7.3.1 closes with the strict-contract corpus accepted for v8.0 SFT training.
The final full run (`realistic-v7-full-v5-1-pr41-20260523T101100Z`) passed the
hard corpus gate with a 0.5549% drop rate.

The residual drops are not blocking v8.0, but they should be investigated before
future corpus expansion or another teacher-backed production run.

## Residual investigation 1 — `kernel_forward:after-rewind`

- **Observed**: 5 drops out of 29 `kernel_forward:after-rewind` scenarios.
- **Failure mode**: target mismatch; teacher predicted `kernel_goto(cursor.kind=ref)`
  where the scenario target was `kernel_forward`.
- **Likely cause**: wording bias around refs in an after-rewind context. The
  teacher treats a visible ref as a navigation target instead of continuing the
  temporal cursor forward.
- **v8.x action**: run a focused wording spike before reusing or expanding this
  template. If it remains unstable, classify it with other policy-sensitive
  navigation templates rather than silently accepting mixed behavior.

## Residual investigation 2 — teacher truncation

- **Observed**: 4 drops with `teacher_finish_reason=length`.
- **Failure mode**: `teacher_truncation` after `max_completion_tokens=8192`,
  with `content_len` in the 8309-8550 range.
- **Confirmed cause in PR #46 Phase 0**: the original OpenAI schema forced
  terminal actions to include tool-call-only fields. `stop(answer_ready)` could
  therefore get stuck satisfying the large `arguments` shape until
  `finish_reason=length`.
- **Decision**: use a flat nullable OpenAI wire DTO and enforce variant
  consistency in the adapter mapper. See
  `docs/training/openai-wire-schema-decision-2026-05-23.md`.

## v8.1 — action_correctness metric

Status: implemented.

v8.1 adds a per-field `action_correctness` metric while preserving the legacy
`exact_match` score. Field correctness policy lives in
`operator-shared-domain`, so the evaluator reports the contract's own scoring
rules instead of maintaining private string-key scoring maps.

The v8.0 artifacts rescored under this metric show:

- Frontier `gpt-4o-mini`: 45.87% action correctness, 37.19% exact match.
- Qwen 0.5B LoRA: 81.40% action correctness, 74.38% exact match.
- Trained model shape invalid rate: 0.00%.
- Remaining trained-model blockers are concentrated in write payload structure:
  `memory.entries[*]`, `memory.relations[*]`, `memory.evidence[*]`,
  `memory.dimensions[*]` and `related[*]`.

Important contract note: generated IDs are currently scored as schema-valid
non-empty strings because the action domain models them as non-empty strings.
If v8.x requires UUID-only generated IDs, add a typed UUID/generated-ID value
object first, then tighten the correctness rule.

## v8.1.1 — constrained decoding for trained-model prediction

v8.0 closure was done without constrained decoding for both frontier and trained
predictions. This is apples-to-apples, but it is not the cleanest methodology.
A real comparison should have both sides enforcing the same structured action
schema.

Two paths for v8.1:

1. **Add outlines/xgrammar to `predict_operator_sft.py`** (~2-3h):
   load an `operator_action_schema.json` equivalent and use structured local
   generation in the inference path. This is local, reproducible and has no
   serving dependency.
2. **Deploy vLLM with LoRA + guided decoding** (~1-2h if infra is ready):
   run a real backend for `0.5b.llm.underpassai.com`, then use the same
   OpenAI-compatible API path for interactive use and evaluation.

Decision: choose after reviewing whether the v8.0 write-action exactness gap is
capacity, decoding or evaluation normalization.

## v8.1.2 — DPO and data prep for write-action correctness

The trained 0.5B is contract-valid on every eval row, but exact match remains
poor for write payloads:

- `kernel_ingest`: 0/19 exact
- `kernel_write_memory`: 1/22 exact

The first hypothesis is that long structured payload copying is not being
evaluated at the right abstraction level. The second is that the model needs
either constrained decoding or deterministic prepared-payload resolution during
prediction. Do not jump to a 1.5B fallback until these are separated.

The action_correctness report gives the first concrete target list for v8.1.2:

1. `kernel_ingest`: fix `memory.entries[*]`, `memory.relations[*]`,
   `memory.evidence[*]` and `memory.dimensions[*]` reconstruction.
2. `kernel_write_memory`: fix `related[*]` selection/copying.
3. `kernel_near`: investigate the 3 remaining `anchor` mismatches.
4. `kernel_forward`: investigate the 2 remaining `window` mismatches.

The likely next step is DPO or targeted contrastive data for these exact field
failures, not another broad SFT pass.

### v8.1.2 closure note

v8.1.2 ships the corrected-budget SFT v2 adapter, not the DPO adapter:

- shipped adapter: `/tmp/operator-qwen05-lora-v8.1.2-sft-v2`
- adapter_sha256:
  `43186fa848c5f0e9d71915023f8f01c2341042de8aaf57b0c3c0c574a0f44379`
- action_correct: 226/317 (71.29%)
- valid predictions: 317/317

The DPO experiment is archived as a failed run. It achieved strong preference
metrics during training but degraded inference generation:

- final DPO valid predictions: 194/317
- final DPO action_correct, adjusted over all eval rows: 160/317 (50.47%)
- DPO epoch-1 checkpoint valid predictions: 208/317
- DPO epoch-1 action_correct, adjusted over all eval rows: 164/317 (51.74%)

Both DPO checkpoints are worse than SFT v2 and should not be promoted.

## v8.2 priorities - post v8.1.2 closure

1. **DPO retry with safer perturbation design**:
   - no cross-cut perturbations until a single-class experiment proves they do
     not create shallow shortcuts
   - `spurious_extra_field` must not be applied to all rows by default
   - test perturbation classes in isolation before combining
2. **DPO retry with safer hyperparameters**:
   - `beta >= 0.5`
   - learning rate `<= 1e-6`
   - one epoch by default
   - early stopping on eval reward margin
   - explicit KL monitoring as a hard gate
3. **Constrained decoding via Outlines/XGrammar**:
   - eliminate malformed JSON and invalid action shapes regardless of model
     state
   - make trained-model comparison cleaner against frontier structured output
4. **Alternative preference optimization**:
   - evaluate KTO or IPO if vanilla DPO remains unstable
5. **Action-correctness metric refinement**:
   - add multiset comparison for order-insensitive arrays where production
     semantics do not require ordering (`entries[]`, `relations[]`,
     `evidence[]`, `dimensions[]`, `related[]`)
   - keep exact matching for fields where order is contract-relevant

## v8.2 — semantic scoring for permissive text

`CorrectnessMode::Permissive` currently checks that free-text fields are
present and type-correct. This is intentionally conservative for v8.1: it avoids
pretending that byte-exact matching is meaningful for user-facing prose or
queries, while still rejecting empty output.

v8.2 should replace the permissive rule with semantic similarity once there is
a stable embedding service and a calibrated threshold. Candidate fields:

- `kernel_ask.arguments.query`
- `stop.answer`
- `kernel_write_memory.summary`
- `kernel_write_memory.body`

## Cluster cleanup

- `0.5b.llm.underpassai.com` ingress was prepared during v8.0 but does not by
  itself prove a live 0.5B vLLM backend is serving the trained adapter. Either
  deploy a real 0.5B vLLM backend in v8.1 or remove the ingress host.
- `underpass-llm-gemma-4-31b-structured` was scaled to zero during v8.0
  training to free GPUs. Restore it if any downstream service still depends on
  that deployment.
- Historical scratch/evidence files still outside git:
  `docs/training/viability_pack_gpt55.txt`,
  `docs/training/operator-v8-0-sft-closure-audit-2026-05-23.md` and
  `docs/training/operator-v8-0-state-explainer-2026-05-23.md`.
