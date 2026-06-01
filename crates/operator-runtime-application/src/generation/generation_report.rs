//! Outcome of a window-expansion generation run: the accepted SFT
//! trajectories plus an audit of every dropped episode.

use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;

use crate::generation::episode_drop::EpisodeDrop;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationReport {
    trajectories: Vec<TrainingTrajectory>,
    drops: Vec<EpisodeDrop>,
    accepted_episodes: usize,
}

impl GenerationReport {
    pub fn new(
        trajectories: Vec<TrainingTrajectory>,
        drops: Vec<EpisodeDrop>,
        accepted_episodes: usize,
    ) -> Self {
        Self {
            trajectories,
            drops,
            accepted_episodes,
        }
    }

    /// All trajectories from accepted episodes (one per non-error step plus the
    /// terminal stop), flattened across the run.
    pub fn trajectories(&self) -> &[TrainingTrajectory] {
        &self.trajectories
    }

    pub fn drops(&self) -> &[EpisodeDrop] {
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

    /// Consumes the report, yielding the accepted trajectories for the dataset
    /// writer.
    pub fn into_trajectories(self) -> Vec<TrainingTrajectory> {
        self.trajectories
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_runtime_domain::session::window_coverage_outcome::WindowCoverageOutcome;

    #[test]
    fn aggregates_counts_and_drop_rate() {
        let drops = vec![EpisodeDrop::new(
            "about:a".to_string(),
            WindowCoverageOutcome::Incomplete { remaining: 1 },
            false,
            true,
            true,
        )];
        let report = GenerationReport::new(Vec::new(), drops, 3);
        assert_eq!(report.accepted_episodes(), 3);
        assert_eq!(report.dropped_episodes(), 1);
        assert_eq!(report.total_episodes(), 4);
        assert!((report.drop_rate() - 0.25).abs() < f64::EPSILON);
        assert!(report.trajectories().is_empty());
        assert!(report.into_trajectories().is_empty());
    }

    #[test]
    fn drop_rate_is_zero_for_an_empty_run() {
        let report = GenerationReport::new(Vec::new(), Vec::new(), 0);
        assert!(report.drop_rate().abs() < f64::EPSILON);
        assert_eq!(report.total_episodes(), 0);
    }
}
