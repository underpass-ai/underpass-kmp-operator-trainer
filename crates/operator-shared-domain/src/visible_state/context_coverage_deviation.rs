//! `ContextCoverageDeviation` is the accumulated, online estimate of how far
//! the retrieved context still is from the context the task needs — computed
//! with no gold labels from [`super::navigation_signals::NavigationSignals`] at
//! each step of a multi-step KMP navigation. It drives the policy's
//! expand-vs-stop decision and is surfaced into [`super::visible_state::VisibleState`].
//!
//! `deviation` is in `[0.0, 1.0]` (1.0 = nothing covered, 0.0 = fully covered)
//! and ratchets monotonically DOWN as context accumulates — mirroring
//! `known_refs` only ever growing — except a sticky `conflict_blocking` flag,
//! which blocks "stop" regardless of deviation until conflicts are resolved.
//!
//! The six sub-scores and their weights are a calibration target, not magic
//! constants: once this field is in the visible state, weights/thresholds are
//! tuned offline against the testkit oracle evidence-hit.

use crate::visible_state::coverage_deviation_snapshot::CoverageDeviationSnapshot;
use crate::visible_state::navigation_signals::NavigationSignals;

const WEIGHT_DIM: f64 = 0.20;
const WEIGHT_PAGE: f64 = 0.15;
const WEIGHT_PROOF: f64 = 0.25;
const WEIGHT_CONFLICT: f64 = 0.15;
const WEIGHT_FRONTIER: f64 = 0.15;
const WEIGHT_ANSWER: f64 = 0.10;

const EWMA_ALPHA: f64 = 0.5;
const CONFLICT_SATURATION: f64 = 3.0;
const FRONTIER_HALF_LIFE: f64 = 2.0;
const SATURATION_STREAK: u32 = 2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContextCoverageDeviation {
    deviation: f64,
    dim_gap: f64,
    page_gap: f64,
    proof_gap: f64,
    conflict_gap: f64,
    frontier_gap: f64,
    answer_gap: f64,
    saturated: bool,
    conflict_blocking: bool,
    last_yield: u32,
    stall_streak: u32,
}

impl ContextCoverageDeviation {
    /// Start of a session: nothing observed yet, so deviation is maximal.
    pub fn initial() -> Self {
        Self {
            deviation: 1.0,
            dim_gap: 0.0,
            page_gap: 0.0,
            proof_gap: 0.0,
            conflict_gap: 0.0,
            frontier_gap: 0.0,
            answer_gap: 0.0,
            saturated: false,
            conflict_blocking: false,
            last_yield: 0,
            stall_streak: 0,
        }
    }

    /// Fold one step's signals into the running estimate. `new_refs` is the
    /// count of refs this step added that the operator did not already know
    /// (the marginal-information yield).
    #[must_use]
    pub fn observing(&self, signals: &NavigationSignals, new_refs: u32) -> Self {
        let dim_gap = ratio(
            signals.coverage_missing(),
            signals.coverage_requested().max(1),
        );
        let page_gap = if signals.page_has_more() {
            ratio(
                signals.page_total().saturating_sub(signals.page_returned()),
                signals.page_total().max(1),
            )
        } else {
            0.0
        };
        let proof_gap = if signals.confidence_is_unknown() {
            1.0
        } else {
            let frontier = f64::from(signals.frontier_size());
            frontier / (frontier + FRONTIER_HALF_LIFE)
        };
        let conflict_gap = clamp01(f64::from(signals.conflicts()) / CONFLICT_SATURATION);
        let frontier_gap = ratio(signals.unvisited_links(), signals.total_links().max(1));
        let answer_gap = if signals.answer_is_null() { 1.0 } else { 0.0 };

        let step_deviation = WEIGHT_DIM * dim_gap
            + WEIGHT_PAGE * page_gap
            + WEIGHT_PROOF * proof_gap
            + WEIGHT_CONFLICT * conflict_gap
            + WEIGHT_FRONTIER * frontier_gap
            + WEIGHT_ANSWER * answer_gap;

        // EWMA, ratcheted down: accumulating context can only reduce perceived
        // deviation; a noisy high step never raises it.
        let ewma = EWMA_ALPHA * step_deviation + (1.0 - EWMA_ALPHA) * self.deviation;
        let deviation = self.deviation.min(ewma);

        // Saturation: expanding stopped yielding new refs AND the dimensional
        // shortfall stopped shrinking, for SATURATION_STREAK consecutive steps.
        let stall_streak = if new_refs == 0 && dim_gap >= self.dim_gap {
            self.stall_streak.saturating_add(1)
        } else {
            0
        };

        Self {
            deviation,
            dim_gap,
            page_gap,
            proof_gap,
            conflict_gap,
            frontier_gap,
            answer_gap,
            saturated: stall_streak >= SATURATION_STREAK,
            conflict_blocking: self.conflict_blocking || signals.conflicts() > 0,
            last_yield: new_refs,
            stall_streak,
        }
    }

    pub fn deviation(&self) -> f64 {
        self.deviation
    }

    pub fn dim_gap(&self) -> f64 {
        self.dim_gap
    }

    pub fn page_gap(&self) -> f64 {
        self.page_gap
    }

    pub fn proof_gap(&self) -> f64 {
        self.proof_gap
    }

    pub fn conflict_gap(&self) -> f64 {
        self.conflict_gap
    }

    pub fn frontier_gap(&self) -> f64 {
        self.frontier_gap
    }

    pub fn answer_gap(&self) -> f64 {
        self.answer_gap
    }

    pub fn saturated(&self) -> bool {
        self.saturated
    }

    pub fn conflict_blocking(&self) -> bool {
        self.conflict_blocking
    }

    pub fn last_yield(&self) -> u32 {
        self.last_yield
    }

    /// Quantized, `Eq`-able summary for the visible state the policy perceives.
    /// The full `f64` estimate stays the runtime loop's accumulator.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn snapshot(&self) -> CoverageDeviationSnapshot {
        let milli = (self.deviation.clamp(0.0, 1.0) * 1000.0).round() as u16;
        CoverageDeviationSnapshot::new(milli, self.saturated, self.conflict_blocking)
    }
}

fn ratio(numerator: u32, denominator: u32) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        f64::from(numerator) / f64::from(denominator)
    }
}

fn clamp01(value: f64) -> f64 {
    value.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fully_covered() -> NavigationSignals {
        // No missing dimensions, no more pages, no frontier, medium confidence,
        // answer present, all links visited, no conflicts.
        NavigationSignals::new(2, 0, false, 4, 4, 0, 0, false, false, 0, 3)
    }

    fn wide_open(missing: u32) -> NavigationSignals {
        // A large shortfall: missing dimensions, more pages, unresolved proof,
        // unvisited links, unknown answer.
        NavigationSignals::new(3, missing, true, 2, 10, 4, 0, true, true, 5, 6)
    }

    fn is_zero(value: f64) -> bool {
        value.abs() < 1e-9
    }

    #[test]
    fn initial_is_maximal_deviation() {
        let deviation = ContextCoverageDeviation::initial();
        assert!((deviation.deviation() - 1.0).abs() < 1e-9);
        assert!(!deviation.saturated());
        assert!(!deviation.conflict_blocking());
    }

    #[test]
    fn covering_context_drives_deviation_down() {
        let after = ContextCoverageDeviation::initial().observing(&fully_covered(), 3);
        assert!(after.deviation() < 1.0);
        assert!(is_zero(after.dim_gap()));
        assert!(is_zero(after.page_gap()));
        assert!(is_zero(after.answer_gap()));
    }

    #[test]
    fn deviation_ratchets_down_never_up() {
        let low = ContextCoverageDeviation::initial()
            .observing(&fully_covered(), 3)
            .observing(&fully_covered(), 1);
        // A subsequent wide-open step must NOT raise the ratcheted deviation.
        let after_spike = low.observing(&wide_open(2), 0);
        assert!(after_spike.deviation() <= low.deviation());
    }

    #[test]
    fn conflicts_set_a_sticky_blocking_flag() {
        let with_conflict = NavigationSignals::new(2, 0, false, 4, 4, 0, 2, false, false, 0, 3);
        let after = ContextCoverageDeviation::initial().observing(&with_conflict, 1);
        assert!(after.conflict_blocking());
        // Stays blocking even after a clean step.
        let still = after.observing(&fully_covered(), 1);
        assert!(still.conflict_blocking());
    }

    #[test]
    fn no_new_refs_and_no_progress_marks_saturated() {
        let signals = wide_open(2);
        let after = ContextCoverageDeviation::initial()
            .observing(&signals, 0)
            .observing(&signals, 0);
        assert!(after.saturated());
        assert_eq!(after.last_yield(), 0);
    }
}
