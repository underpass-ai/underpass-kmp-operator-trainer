//! Outcome of a cross-about count generation run: the accepted SFT trajectories
//! plus an audit of every dropped episode. Mirrors
//! [`crate::generation::generation_report::GenerationReport`] with cross-about
//! drops.

use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;

use crate::generation::cross_about_drop::CrossAboutDrop;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrossAboutGenerationReport {
    trajectories: Vec<TrainingTrajectory>,
    drops: Vec<CrossAboutDrop>,
    accepted_episodes: usize,
}

impl CrossAboutGenerationReport {
    pub fn new(
        trajectories: Vec<TrainingTrajectory>,
        drops: Vec<CrossAboutDrop>,
        accepted_episodes: usize,
    ) -> Self {
        Self {
            trajectories,
            drops,
            accepted_episodes,
        }
    }

    pub fn trajectories(&self) -> &[TrainingTrajectory] {
        &self.trajectories
    }

    pub fn drops(&self) -> &[CrossAboutDrop] {
        &self.drops
    }

    pub fn accepted_episodes(&self) -> usize {
        self.accepted_episodes
    }

    pub fn dropped_episodes(&self) -> usize {
        self.drops.len()
    }

    pub fn total_episodes(&self) -> usize {
        self.accepted_episodes + self.drops.len()
    }

    /// Fraction of episodes dropped (0.0 when there were no episodes). The CLI
    /// gates the run on this against a configured maximum.
    #[allow(clippy::cast_precision_loss)]
    pub fn drop_rate(&self) -> f64 {
        let total = self.total_episodes();
        if total == 0 {
            0.0
        } else {
            self.dropped_episodes() as f64 / total as f64
        }
    }

    pub fn into_trajectories(self) -> Vec<TrainingTrajectory> {
        self.trajectories
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_runtime_domain::session::cross_about_coverage_outcome::CrossAboutCoverageOutcome;

    #[test]
    fn aggregates_counts_and_drop_rate() {
        let drops = vec![CrossAboutDrop::new(
            "about:eu".to_string(),
            CrossAboutCoverageOutcome::Complete,
            true,
        )];
        let report = CrossAboutGenerationReport::new(Vec::new(), drops, 3);
        assert_eq!(report.accepted_episodes(), 3);
        assert_eq!(report.dropped_episodes(), 1);
        assert_eq!(report.total_episodes(), 4);
        assert!((report.drop_rate() - 0.25).abs() < f64::EPSILON);
        assert!(report.into_trajectories().is_empty());
    }
}
