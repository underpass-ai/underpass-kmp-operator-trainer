//! Record of a cross-about count episode the generator dropped instead of
//! emitting as training data.
//!
//! Mirrors [`crate::generation::episode_drop::EpisodeDrop`] but for cross-about
//! coverage: the drop reason is either a surfaced conflict (which would force an
//! Escalate terminal a count corpus excludes) or the set of abouts the session
//! left short of their gold operands.

use operator_runtime_domain::session::cross_about_coverage_outcome::CrossAboutCoverageOutcome;
use operator_shared_domain::ids::about_id::AboutId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossAboutDrop {
    entry_about: String,
    outcome: CrossAboutCoverageOutcome,
    conflict_blocking: bool,
}

impl CrossAboutDrop {
    pub fn new(
        entry_about: String,
        outcome: CrossAboutCoverageOutcome,
        conflict_blocking: bool,
    ) -> Self {
        Self {
            entry_about,
            outcome,
            conflict_blocking,
        }
    }

    pub fn entry_about(&self) -> &str {
        &self.entry_about
    }

    pub fn outcome(&self) -> &CrossAboutCoverageOutcome {
        &self.outcome
    }

    pub fn conflict_blocking(&self) -> bool {
        self.conflict_blocking
    }

    /// Why the episode was dropped: a surfaced conflict takes precedence; then
    /// the abouts left short of their gold operands.
    pub fn reason(&self) -> String {
        if self.conflict_blocking {
            return "conflict_blocking".to_string();
        }
        match &self.outcome {
            CrossAboutCoverageOutcome::Incomplete { uncovered_abouts } => {
                let abouts = uncovered_abouts
                    .iter()
                    .map(AboutId::as_str)
                    .collect::<Vec<_>>()
                    .join(",");
                format!("uncovered_abouts:{abouts}")
            }
            CrossAboutCoverageOutcome::Complete => "complete".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::ids::about_id::AboutId;

    #[test]
    fn reports_uncovered_abouts_as_the_reason() {
        let drop = CrossAboutDrop::new(
            "about:eu".to_string(),
            CrossAboutCoverageOutcome::Incomplete {
                uncovered_abouts: vec![
                    AboutId::parse("about:us").unwrap(),
                    AboutId::parse("about:apac").unwrap(),
                ],
            },
            false,
        );
        assert_eq!(drop.entry_about(), "about:eu");
        assert_eq!(drop.reason(), "uncovered_abouts:about:us,about:apac");
    }

    #[test]
    fn a_surfaced_conflict_takes_precedence() {
        let drop = CrossAboutDrop::new(
            "about:eu".to_string(),
            CrossAboutCoverageOutcome::Complete,
            true,
        );
        assert!(drop.conflict_blocking());
        assert_eq!(drop.reason(), "conflict_blocking");
    }
}
