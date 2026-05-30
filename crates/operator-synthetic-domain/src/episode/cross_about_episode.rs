//! One authored cross-about count episode: a count-over-period question whose
//! operands are distributed across several abouts, driven through the multi-step
//! loop to produce SFT trajectories.
//!
//! Where a [`crate::episode::window_expansion_episode::WindowExpansionEpisode`]
//! covers a period inside a single about, a cross-about episode names several
//! [`CrossAboutTarget`]s — one per about the session must visit. The session
//! enters at the first target's about, then `kernel_wake`s into each remaining
//! about and widens its window until every target's gold entries are retrieved.
//!
//! Coverage is gold-only here: across abouts the kernel's last-move in-band
//! signals reflect just the final about, so the per-about gold set is the
//! authoritative completeness check (see the cross-about oracle). The numeric
//! count itself is computed downstream by the reader, not by the operator — an
//! episode carries which entries are in the period, not how many.

use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;

use crate::episode::cross_about_target::CrossAboutTarget;
use crate::episode::window_expansion_spec::WindowExpansionSpec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossAboutEpisode {
    targets: Vec<CrossAboutTarget>,
    goal: TrajectoryGoal,
    spec: WindowExpansionSpec,
    token_budget: u32,
}

impl CrossAboutEpisode {
    /// Build an episode. `targets` must be non-empty — the entry about is the
    /// first target's; the mapper that builds episodes from external data
    /// enforces this before constructing.
    pub fn new(
        targets: Vec<CrossAboutTarget>,
        goal: TrajectoryGoal,
        spec: WindowExpansionSpec,
        token_budget: u32,
    ) -> Self {
        Self {
            targets,
            goal,
            spec,
            token_budget,
        }
    }

    pub fn targets(&self) -> &[CrossAboutTarget] {
        &self.targets
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

    /// The about the session enters at — the first target's. Requires at least
    /// one target (guaranteed by the mapper for externally authored episodes).
    pub fn entry_about(&self) -> &AboutId {
        self.targets
            .first()
            .expect("a cross-about episode has at least one target")
            .about()
    }

    /// The union of every target's gold entries: the overall operand set the
    /// session must retrieve across all abouts.
    pub fn expected_refs(&self) -> Vec<MemoryRef> {
        let mut refs = Vec::new();
        for target in &self.targets {
            for memory_ref in target.expected_refs() {
                if !refs.contains(memory_ref) {
                    refs.push(memory_ref.clone());
                }
            }
        }
        refs
    }

    /// Whether every target carries a gold set. Cross-about coverage is gold-only
    /// (last-move signals only reflect the final about), so a target without gold
    /// cannot be verified — the mapper rejects such episodes.
    pub fn has_gold(&self) -> bool {
        !self.targets.is_empty() && self.targets.iter().all(|t| !t.expected_refs().is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::value_objects::positive_count::PositiveCount;

    fn spec() -> WindowExpansionSpec {
        WindowExpansionSpec::new(
            PositiveCount::parse(4, "window").unwrap(),
            PositiveCount::parse(5, "iterations").unwrap(),
        )
    }

    fn target(about: &str, refs: &[&str]) -> CrossAboutTarget {
        CrossAboutTarget::new(
            AboutId::parse(about).unwrap(),
            refs.iter().map(|r| MemoryRef::parse(*r).unwrap()).collect(),
        )
    }

    #[test]
    fn entry_about_is_the_first_target_and_gold_is_the_union() {
        let episode = CrossAboutEpisode::new(
            vec![
                target("about:eu", &["node:eu1", "node:eu2"]),
                target("about:us", &["node:us1"]),
            ],
            TrajectoryGoal::parse("Count workshops across EU and US.").unwrap(),
            spec(),
            8192,
        );
        assert_eq!(episode.entry_about().as_str(), "about:eu");
        assert_eq!(episode.expected_refs().len(), 3);
        assert!(episode.has_gold());
        assert_eq!(episode.token_budget(), 8192);
    }

    #[test]
    fn has_gold_is_false_when_a_target_lacks_gold() {
        let episode = CrossAboutEpisode::new(
            vec![
                target("about:eu", &["node:eu1"]),
                target("about:us", &[]),
            ],
            TrajectoryGoal::parse("Count.").unwrap(),
            spec(),
            8192,
        );
        assert!(!episode.has_gold());
    }
}
