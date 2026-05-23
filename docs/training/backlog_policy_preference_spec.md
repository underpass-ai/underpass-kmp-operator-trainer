# Backlog — Policy Preference Spec design

## Context

The following templates were removed in PR #40 (v7.3 closure) because they test
policy preferences that the strict action contract does not enforce, and no
capable LLM teacher (`gpt-4o-mini`, `gpt-5.1`, `gpt-5.2`, `gpt-5.5`) consistently
chose the expected target at deterministic settings.

## Removed templates

### `escalate:do-not-speculate`

- Tests: model should prefer escalate over `kernel_ask` when context is speculative.
- Why removed: capable models picked different wrong tools during the viability spike.
- Future spec: would need to formalize speculation detection in visible state or contract.

### `escalate:budget-alternative`

- Tests: model should escalate when budget is constrained.
- Why removed: capable models picked varied tools under the same scenario.
- Future spec: would need explicit budget-state policy in contract.

### `escalate:no-traceable-path`

- Tests: model should escalate when no traceable path exists.
- Why removed: capable models picked `kernel_trace` or `kernel_wake` instead.
- Future spec: needs explicit untraceability marker in visible state.

### `escalate:ambiguous-scope`

- Tests: model should escalate when scope is ambiguous.
- Why removed: capable models treated clarification or tracing as plausible bounded actions.
- Future spec: needs scope-ambiguity formalization.

### `kernel_goto:temporal-cursor`

- Tests: model should choose a direct temporal `kernel_goto` over temporal
  movement actions.
- Why removed: after the one permitted wording fix, `gpt-4o-mini` still selected
  `kernel_forward`, showing the distinction is not stable enough for v7.3 corpus
  production.
- Future spec: needs a formal distinction between direct temporal jump and
  relative temporal movement.

### `stop:after-escalate-attempt`

- Tests: model should terminate with `stop(no_candidate)` after prior escalation
  failed to produce an executable candidate.
- Why removed: after escalation templates were removed from the strict-contract
  corpus, this template still depended on escalation history and consistently
  triggered policy-preference behavior.
- Future spec: needs an explicit representation of exhausted escalation attempts
  and terminal no-candidate policy.

### `stop:premature-ask-temptation`

- Tests: model should prefer `stop(no_candidate)` over another bounded query when
  visible memory cannot produce a valid answer.
- Why removed: full-run sampling showed systematic selection of
  `stop(answer_ready)` or other policy-preference behavior rather than stable
  strict-contract behavior.
- Future spec: needs a formal distinction between answer-ready and no-candidate
  terminal states.

### `stop:no-candidate` (removed in v7.3.1, PR #41)

- **Tests**: model should pick `Stop(reason=NoCandidate)` when no visible ref
  produces a valid answer, even when budget allows another call.
- **Why removed**: empirically unstable teacher behavior at T=0 (60% pass rate,
  theme-dependent). Contract does not enforce this behavior: `kernel_ask` is
  contract-valid when `budget.calls_remaining > 0` and tools include ask.
- **Empirical evidence**:
  - Full v5 run: 18/28 accepted, 10 dropped.
  - Drops uncorrelated with budget (drops at `calls_remaining` 1, 2, 4, 8).
  - Drop concentration by topic: `bug:worker-retry-storm` -> 6/6 drops, other
    topics partial pass.
  - Capable T=0 models (`gpt-5.1`, `gpt-5.2`) handled the n=1 viability spikes;
    canonical teacher (`gpt-4o-mini`) does not produce it reliably.
- **Consequence**: v8.0 SFT'd model will not see any
  `StopReason::NoCandidate` examples. The variant becomes effectively dead in
  production output. This is an explicit, documented trade-off accepted for
  v7.3.1 closure.

### Required For v8.x Prescriptive Spec Design

A `NoCandidateSpec` or equivalent must:

1. Detect contexts where no visible ref produces a valid answer.
   - Probable requirement: confidence/relevance scoring in `VisibleState`, which
     does not exist today.
   - Alternative: explicit `NoCandidateMarker` VO set by scenario generator when
     the template knows no answer is available.

2. Generate corpus examples via the existing `subject.prepared_action()`
   fast-path mechanism. Bypasses teacher unreliability entirely.

3. Restore `stop:no-candidate` as a deterministic template with theme-balanced
   coverage, avoiding the observed `bug:worker-retry-storm` topic bias.

The `escalate:*` removals and `stop:no-candidate` removal together form the
catalog of prescriptive policy that v8.x's prescriptive spec design must recover.
No prescriptive behavior is currently trained.

## Architectural Gap — Prescriptive Contract Specs

The current strict action contract is **rejection-only**: every spec under
`crates/operator-shared-domain/src/specifications/` rejects invalid actions,
none requires a specific action shape. This is sufficient for rejection-style
invariants (budget, mode, refs, ingest shape), but cannot encode "in context X
the action MUST be escalate" or similar prescriptions.

The 4 escalate templates removed in PR #40 were attempting to encode such
prescription without infrastructure to back it. The empirical spike confirmed
that no LLM teacher (`gpt-4o-mini`, `gpt-5.1`, `gpt-5.2`, `gpt-5.5`) reliably
produces escalate from goal text, for two reasons:

1. RLHF rewards helpfulness, not delegation or self-awareness of limits.
2. The contract gives no signal that escalate is required, so the teacher's
   "explore-tools-first" default is permitted and rewarded.

### Required Infrastructure

To enable corpus-level training of escalate (and other prescriptive behaviors),
the architecture needs a new concept distinct from `Specification<Subject>`:

```rust
pub trait ContractRequirement {
    fn required_action(
        &self,
        subject: &ActionContractSubject,
    ) -> Option<RequiredActionShape>;
}

pub struct RequiredActionShape {
    kind: OperatorActionKind,
    reason: Option<EscalateReason | StopReason>,
    tool: Option<KmpMcpCapability>,
}
```

A composite requirement engine evaluates all `ContractRequirement`s for a subject
and produces zero or one `RequiredActionShape`. Multiple conflicting
requirements are a design error caught at spec composition.

### Integration Points

1. Strict validator: a required-action-shape subject whose action does not match
   the required shape produces `ContractViolationCode::ActionMismatch`.
2. Corpus generator: scenarios with `RequiredActionShape::Some` override the
   teacher with `prepared_action` carrying the required shape. The teacher never
   sees those subjects; the corpus row reflects what the contract demands, not
   what an LLM would choose.
3. Calibration: distinguishes "model violated a rejection rule" from "model
   failed to meet a requirement." Both fail calibration but for different
   reasons; the operator-trained 0.5B improves on the second class only with
   prescription-aware training data.

### `EscalateReason` Variants And Candidate Requirements

- `AmbiguousIntent`: subject has multiple plausible goals -> required if some
  `IntentAmbiguityMarker` is set in visible state.
- `BeyondCapability`: subject's task requires capabilities outside
  `OperatorMode::AllowedTools` and outside escalation target's mode -> unsure if
  this case exists; needs design.
- `LowConfidence`: subject's candidate refs all carry confidence scores below
  threshold -> requires confidence scoring in visible state, which does not exist
  today.

### When To Design

After v7.3 closes and v8.0 SFT training of the 0.5B begins. We will know from
training metrics what prescriptive behaviors the trained model fundamentally
cannot learn from the current rejection-only corpus, and that empirical signal
will drive the prescriptive spec design.
