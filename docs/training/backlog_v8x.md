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

