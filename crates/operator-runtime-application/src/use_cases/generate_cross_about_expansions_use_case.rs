//! Drive a batch of cross-about count episodes through the multi-step loop and
//! collect the SFT trajectories they demonstrate.
//!
//! For each episode the generator compiles a cross-about [`OperatorRequest`],
//! runs the multi-step session (the teacher demonstrates: wake into each about
//! and widen its window until covered, then stop with the union), verifies with
//! the [`CrossAboutOracle`] that *every* about's gold operands were retrieved,
//! and — only when coverage held *and* no conflict surfaced — expands the
//! transcript into trajectories. Each trajectory is attributed to the about that
//! was current at its step (the expander reads the per-step about), so the SFT
//! rows teach the about-switching policy faithfully.
//!
//! Coverage here is gold-only (cross-about signals only reflect the final
//! about), so an episode that stopped short of any about's gold is dropped with
//! its uncovered abouts recorded rather than poisoning the corpus.

use operator_runtime_domain::session::operator_session_id::OperatorSessionId;
use operator_shared_domain::value_objects::task_family::TaskFamily;
use operator_synthetic_domain::episode::cross_about_episode::CrossAboutEpisode;

use crate::errors::generate_window_expansions_error::GenerateWindowExpansionsError;
use crate::generation::cross_about_drop::CrossAboutDrop;
use crate::generation::cross_about_generation_report::CrossAboutGenerationReport;
use crate::services::cross_about_episode_compiler::CrossAboutEpisodeCompiler;
use crate::services::cross_about_oracle::CrossAboutOracle;
use crate::use_cases::expand_session_transcript_use_case::ExpandSessionTranscriptUseCase;
use crate::use_cases::run_operator_session_multi_step_use_case::RunOperatorSessionMultiStepUseCase;

#[derive(Debug)]
pub struct GenerateCrossAboutExpansionsUseCase {
    session: RunOperatorSessionMultiStepUseCase,
    task_family: TaskFamily,
}

impl GenerateCrossAboutExpansionsUseCase {
    pub fn new(session: RunOperatorSessionMultiStepUseCase, task_family: TaskFamily) -> Self {
        Self {
            session,
            task_family,
        }
    }

    pub fn execute(
        &self,
        episodes: &[CrossAboutEpisode],
    ) -> Result<CrossAboutGenerationReport, GenerateWindowExpansionsError> {
        let mut trajectories = Vec::new();
        let mut drops = Vec::new();
        let mut accepted_episodes = 0usize;

        for (index, episode) in episodes.iter().enumerate() {
            let session_id = OperatorSessionId::parse(format!("xab:{index:04}")).map_err(|err| {
                GenerateWindowExpansionsError::SessionId {
                    index,
                    message: err.to_string(),
                }
            })?;
            let request = CrossAboutEpisodeCompiler::compile(session_id, episode).map_err(|err| {
                GenerateWindowExpansionsError::Compile {
                    index,
                    message: err.to_string(),
                }
            })?;

            let transcript = self.session.execute(&request).map_err(|err| {
                GenerateWindowExpansionsError::Session {
                    index,
                    message: err.to_string(),
                }
            })?;

            let outcome = CrossAboutOracle::verify(episode, &transcript);
            let conflict_blocking = transcript
                .final_visible_state()
                .coverage_deviation()
                .conflict_blocking();

            if outcome.is_covered() && !conflict_blocking {
                let episode_trajectories = ExpandSessionTranscriptUseCase::expand(
                    &request,
                    &transcript,
                    &self.task_family,
                )
                .map_err(|err| GenerateWindowExpansionsError::Expand {
                    index,
                    message: err.to_string(),
                })?;
                trajectories.extend(episode_trajectories);
                accepted_episodes += 1;
            } else {
                drops.push(CrossAboutDrop::new(
                    episode.entry_about().as_str().to_string(),
                    outcome,
                    conflict_blocking,
                ));
            }
        }

        Ok(CrossAboutGenerationReport::new(
            trajectories,
            drops,
            accepted_episodes,
        ))
    }
}
