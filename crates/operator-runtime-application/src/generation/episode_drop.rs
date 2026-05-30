//! Record of a window-expansion episode the generator dropped instead of
//! emitting as training data.

use operator_runtime_domain::session::window_coverage_outcome::WindowCoverageOutcome;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeDrop {
    about: String,
    outcome: WindowCoverageOutcome,
    conflict_blocking: bool,
    gold_covered: bool,
}

impl EpisodeDrop {
    pub fn new(
        about: String,
        outcome: WindowCoverageOutcome,
        conflict_blocking: bool,
        gold_covered: bool,
    ) -> Self {
        Self {
            about,
            outcome,
            conflict_blocking,
            gold_covered,
        }
    }

    pub fn about(&self) -> &str {
        &self.about
    }

    pub fn outcome(&self) -> WindowCoverageOutcome {
        self.outcome
    }

    pub fn conflict_blocking(&self) -> bool {
        self.conflict_blocking
    }

    /// Whether the episode's gold coverage set (if any) was fully retrieved.
    /// Always true for episodes with no gold.
    pub fn gold_covered(&self) -> bool {
        self.gold_covered
    }

    /// Why the episode was dropped: a surfaced conflict takes precedence (it
    /// would force an Escalate terminal, which a count-over-period corpus
    /// excludes); then a gold-coverage miss (the window did not retrieve every
    /// expected period entry); otherwise the coverage shortfall the oracle
    /// reported from the kernel's signals.
    pub fn reason(&self) -> &'static str {
        if self.conflict_blocking {
            "conflict_blocking"
        } else if !self.gold_covered {
            "missing_gold_refs"
        } else {
            self.outcome.as_str()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_parts_and_reports_the_coverage_shortfall_reason() {
        let drop = EpisodeDrop::new(
            "about:p".to_string(),
            WindowCoverageOutcome::Incomplete { remaining: 3 },
            false,
            true,
        );
        assert_eq!(drop.about(), "about:p");
        assert_eq!(
            drop.outcome(),
            WindowCoverageOutcome::Incomplete { remaining: 3 }
        );
        assert!(!drop.conflict_blocking());
        assert!(drop.gold_covered());
        assert_eq!(drop.reason(), "incomplete");
    }

    #[test]
    fn a_gold_coverage_miss_reports_missing_gold_refs() {
        // Signals said covered, but the gold period set was not fully retrieved.
        let drop = EpisodeDrop::new(
            "about:p".to_string(),
            WindowCoverageOutcome::Complete,
            false,
            false,
        );
        assert!(!drop.gold_covered());
        assert_eq!(drop.reason(), "missing_gold_refs");
    }

    #[test]
    fn a_surfaced_conflict_takes_precedence_in_the_reason() {
        let drop = EpisodeDrop::new(
            "about:p".to_string(),
            WindowCoverageOutcome::Complete,
            true,
            true,
        );
        assert!(drop.conflict_blocking());
        assert_eq!(drop.reason(), "conflict_blocking");
    }
}
