//! Deterministic, no-gold verification that a window-expansion session reached
//! coverage completeness.
//!
//! The oracle reads only the kernel's own in-band signals already recorded on
//! the transcript — the last temporal move's [`NavigationSignals`]
//! (`page_has_more`, `frontier_size`, `coverage_missing`) and the final
//! `coverage_deviation` saturation flag — and classifies the session as
//! [`WindowCoverageOutcome::Complete`], [`WindowCoverageOutcome::Saturated`],
//! or [`WindowCoverageOutcome::Incomplete`]. It is a label/verify step the
//! generator uses to accept or drop a transcript; it is **never** an SFT target
//! and never injects an expected action, so the learnable policy is the
//! teacher's demonstrated stop decision, not the oracle's verdict.
//!
//! Coverage here is judged from the kernel's self-reported shortfall, not from
//! a ground-truth count. A stronger oracle that compares against an expected
//! count-over-period total requires kernel ground truth and is left to the
//! generation-run wiring.

use operator_runtime_domain::session::session_transcript::SessionTranscript;
use operator_runtime_domain::session::window_coverage_outcome::WindowCoverageOutcome;

#[derive(Debug)]
pub struct WindowExpansionOracle;

impl WindowExpansionOracle {
    /// Classify how covered the session's retrieved context was. Looks at the
    /// last successful temporal move; if the session never made one, the result
    /// is `Incomplete { remaining: 0 }` (no coverage evidence was gathered).
    pub fn verify(transcript: &SessionTranscript) -> WindowCoverageOutcome {
        let Some(signals) = transcript
            .steps()
            .iter()
            .rev()
            .find_map(|step| step.observation().signals())
        else {
            return WindowCoverageOutcome::Incomplete { remaining: 0 };
        };

        let remaining = signals
            .coverage_missing()
            .saturating_add(signals.frontier_size());

        if !signals.page_has_more() && remaining == 0 {
            WindowCoverageOutcome::Complete
        } else if transcript
            .final_visible_state()
            .coverage_deviation()
            .saturated()
        {
            WindowCoverageOutcome::Saturated
        } else {
            WindowCoverageOutcome::Incomplete { remaining }
        }
    }
}
