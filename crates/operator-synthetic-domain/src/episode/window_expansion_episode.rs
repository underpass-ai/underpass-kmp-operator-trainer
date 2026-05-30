//! One authored window-expansion episode: a count-over-period question the
//! generator will drive through the multi-step loop to produce SFT
//! trajectories.
//!
//! An episode names the memory it is about (`about`), the question/instruction
//! the policy is given (`goal`, which also conveys the period and the "widen
//! until covered" directive), the expansion parameters (`spec`), and the token
//! budget the session may spend. It deliberately carries no session id — the
//! generator assigns those per run so ids stay unique.
//!
//! An episode MAY carry `expected_refs`: the gold set of memory entries that
//! fall in the period the window must cover. When present this is the
//! authoritative coverage check — the generator keeps the trajectory only if
//! the session actually retrieved every one of those refs — and the kernel's
//! in-band coverage signals are a fallback used only when no gold is supplied.
//! Note this is a *coverage* gold (which entries are in the period), not a
//! numeric count: the count itself is computed downstream by the reader, not by
//! the operator.

use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;

use crate::episode::window_expansion_spec::WindowExpansionSpec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowExpansionEpisode {
    about: AboutId,
    goal: TrajectoryGoal,
    spec: WindowExpansionSpec,
    token_budget: u32,
    expected_refs: Vec<MemoryRef>,
}

impl WindowExpansionEpisode {
    pub fn new(
        about: AboutId,
        goal: TrajectoryGoal,
        spec: WindowExpansionSpec,
        token_budget: u32,
    ) -> Self {
        Self {
            about,
            goal,
            spec,
            token_budget,
            expected_refs: Vec::new(),
        }
    }

    /// Attach the gold period-entry set this episode's window must cover.
    #[must_use]
    pub fn with_expected_refs(mut self, expected_refs: Vec<MemoryRef>) -> Self {
        self.expected_refs = expected_refs;
        self
    }

    pub fn about(&self) -> &AboutId {
        &self.about
    }

    pub fn goal(&self) -> &TrajectoryGoal {
        &self.goal
    }

    pub fn spec(&self) -> WindowExpansionSpec {
        self.spec
    }

    pub fn token_budget(&self) -> u32 {
        self.token_budget
    }

    pub fn expected_refs(&self) -> &[MemoryRef] {
        &self.expected_refs
    }

    /// Whether this episode carries a gold coverage set. When false, coverage
    /// is judged from the kernel's in-band signals instead.
    pub fn has_gold(&self) -> bool {
        !self.expected_refs.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::value_objects::positive_count::PositiveCount;

    #[test]
    fn exposes_parts() {
        let spec = WindowExpansionSpec::new(
            PositiveCount::parse(8, "window").unwrap(),
            PositiveCount::parse(5, "iterations").unwrap(),
        );
        let episode = WindowExpansionEpisode::new(
            AboutId::parse("about:period").unwrap(),
            TrajectoryGoal::parse("Count workshops across the last four months.").unwrap(),
            spec,
            4096,
        );
        assert_eq!(episode.about().as_str(), "about:period");
        assert_eq!(episode.spec().max_iterations().as_usize(), 5);
        assert_eq!(episode.token_budget(), 4096);
        assert!(!episode.has_gold());
        assert!(episode.expected_refs().is_empty());
    }

    #[test]
    fn carries_a_gold_coverage_set() {
        let spec = WindowExpansionSpec::new(
            PositiveCount::parse(4, "window").unwrap(),
            PositiveCount::parse(3, "iterations").unwrap(),
        );
        let episode = WindowExpansionEpisode::new(
            AboutId::parse("about:period").unwrap(),
            TrajectoryGoal::parse("Count workshops.").unwrap(),
            spec,
            4096,
        )
        .with_expected_refs(vec![MemoryRef::parse("node:1").unwrap()]);
        assert!(episode.has_gold());
        assert_eq!(episode.expected_refs().len(), 1);
    }
}
