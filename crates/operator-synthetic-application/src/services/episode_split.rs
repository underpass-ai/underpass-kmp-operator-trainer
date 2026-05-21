//! Result of applying an episode split policy.

use operator_synthetic_domain::episode::synthetic_episode_spec::SyntheticEpisodeSpec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeSplit {
    train: Vec<SyntheticEpisodeSpec>,
    eval: Vec<SyntheticEpisodeSpec>,
}

impl EpisodeSplit {
    pub fn new(train: Vec<SyntheticEpisodeSpec>, eval: Vec<SyntheticEpisodeSpec>) -> Self {
        Self { train, eval }
    }

    pub fn train(&self) -> &[SyntheticEpisodeSpec] {
        &self.train
    }

    pub fn eval(&self) -> &[SyntheticEpisodeSpec] {
        &self.eval
    }
}
