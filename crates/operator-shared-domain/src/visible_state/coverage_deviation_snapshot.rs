//! `CoverageDeviationSnapshot` is the wire-facing, `Eq`-able summary of
//! [`super::context_coverage_deviation::ContextCoverageDeviation`] that the
//! operator perceives inside [`super::visible_state::VisibleState`]. The full
//! `f64` estimate (with its six sub-scores) is the runtime loop's accumulator;
//! the visible state only carries this quantized summary so it stays a value
//! object with stable equality and a compact wire/SFT representation.
//!
//! `deviation_milli` is per-mille of `[0.0, 1.0]` (0 = fully covered,
//! 1000 = nothing covered).

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoverageDeviationSnapshot {
    deviation_milli: u16,
    saturated: bool,
    conflict_blocking: bool,
}

impl CoverageDeviationSnapshot {
    pub fn new(deviation_milli: u16, saturated: bool, conflict_blocking: bool) -> Self {
        Self {
            deviation_milli: deviation_milli.min(1000),
            saturated,
            conflict_blocking,
        }
    }

    /// Start of a session: nothing observed yet, so deviation is maximal.
    pub fn unknown() -> Self {
        Self {
            deviation_milli: 1000,
            saturated: false,
            conflict_blocking: false,
        }
    }

    pub fn deviation_milli(&self) -> u16 {
        self.deviation_milli
    }

    pub fn deviation(&self) -> f64 {
        f64::from(self.deviation_milli) / 1000.0
    }

    pub fn saturated(&self) -> bool {
        self.saturated
    }

    pub fn conflict_blocking(&self) -> bool {
        self.conflict_blocking
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_is_maximal() {
        let snapshot = CoverageDeviationSnapshot::unknown();
        assert_eq!(snapshot.deviation_milli(), 1000);
        assert!(!snapshot.saturated());
        assert!(!snapshot.conflict_blocking());
    }

    #[test]
    fn clamps_milli_to_thousand() {
        assert_eq!(
            CoverageDeviationSnapshot::new(5000, false, false).deviation_milli(),
            1000
        );
    }

    #[test]
    fn exposes_fractional_deviation() {
        let snapshot = CoverageDeviationSnapshot::new(350, true, false);
        assert!((snapshot.deviation() - 0.35).abs() < 1e-9);
        assert!(snapshot.saturated());
    }
}
