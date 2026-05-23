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
- **Likely cause**: the non-discriminated structured schema still exposes fields
  such as `answer` on every action shape. The provider can produce a long
  structured response before reaching a parseable action.
- **v8.x action**: diagnose whether this is schema verbosity, model behavior, or
  missing observability. Consider persisting a bounded raw content tail for
  truncation-only drops so future diagnosis does not require another paid call.

## v8.1 — constrained decoding for trained-model prediction

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

## v8.1 — write-action exactness

The trained 0.5B is contract-valid on every eval row, but exact match remains
poor for write payloads:

- `kernel_ingest`: 0/19 exact
- `kernel_write_memory`: 1/22 exact

The first hypothesis is that long structured payload copying is not being
evaluated at the right abstraction level. The second is that the model needs
either constrained decoding or deterministic prepared-payload resolution during
prediction. Do not jump to a 1.5B fallback until these are separated.

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
