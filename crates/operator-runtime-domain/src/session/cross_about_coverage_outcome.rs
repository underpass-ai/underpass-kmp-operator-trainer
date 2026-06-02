//! Verdict of the cross-about oracle: whether a finished cross-about session
//! retrieved every about's gold operand set.
//!
//! Unlike [`crate::session::window_coverage_outcome::WindowCoverageOutcome`],
//! which reads the last temporal move's in-band signals, cross-about coverage
//! cannot be judged from signals alone: the last move only reflects the final
//! about. Completeness is therefore gold-only — every target about's expected
//! entries must be present in the accumulated visible state. Used to accept or
//! drop a transcript before it becomes SFT data; never itself a training target.

use operator_shared_domain::ids::about_id::AboutId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CrossAboutCoverageOutcome {
    /// Every target about's gold entries were retrieved. A stop here is
    /// well-grounded across all abouts.
    Complete,
    /// One or more abouts were left short: their gold entries were not all
    /// retrieved. A stop here is premature; `uncovered_abouts` names them.
    Incomplete { uncovered_abouts: Vec<AboutId> },
}

impl CrossAboutCoverageOutcome {
    /// Whether every about's operand set was retrieved.
    pub fn is_covered(&self) -> bool {
        matches!(self, Self::Complete)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Complete => "complete",
            Self::Incomplete { .. } => "incomplete",
        }
    }

    /// The abouts left short of coverage (empty when complete).
    pub fn uncovered_abouts(&self) -> &[AboutId] {
        match self {
            Self::Complete => &[],
            Self::Incomplete { uncovered_abouts } => uncovered_abouts,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_is_covered_with_no_uncovered_abouts() {
        let outcome = CrossAboutCoverageOutcome::Complete;
        assert!(outcome.is_covered());
        assert_eq!(outcome.as_str(), "complete");
        assert!(outcome.uncovered_abouts().is_empty());
    }

    #[test]
    fn incomplete_lists_the_uncovered_abouts() {
        let outcome = CrossAboutCoverageOutcome::Incomplete {
            uncovered_abouts: vec![AboutId::parse("about:apac").unwrap()],
        };
        assert!(!outcome.is_covered());
        assert_eq!(outcome.as_str(), "incomplete");
        assert_eq!(outcome.uncovered_abouts().len(), 1);
    }
}
