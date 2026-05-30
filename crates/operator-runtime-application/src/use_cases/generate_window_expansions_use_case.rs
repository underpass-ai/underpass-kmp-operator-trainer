//! Drive a batch of window-expansion episodes through the multi-step loop and
//! collect the SFT trajectories they demonstrate.
//!
//! For each episode the generator: compiles it to an [`OperatorRequest`], runs
//! the multi-step session (the injected policy is the teacher demonstrating
//! expansion), verifies coverage with the [`WindowExpansionOracle`], and — only
//! when the session reached coverage *and* surfaced no conflict — expands the
//! transcript into trajectories. Episodes that stopped short of coverage or hit
//! a conflict are dropped with an auditable reason rather than poisoning the
//! corpus with premature-stop or escalate demonstrations.
//!
//! The use case is policy-agnostic: it never injects an expected action and
//! never asks the oracle what to do — the oracle only accepts or drops. What the
//! teacher demonstrated is exactly what the student will learn.

use operator_runtime_domain::session::operator_session_id::OperatorSessionId;
use operator_shared_domain::value_objects::task_family::TaskFamily;
use operator_synthetic_domain::episode::window_expansion_episode::WindowExpansionEpisode;

use crate::errors::generate_window_expansions_error::GenerateWindowExpansionsError;
use crate::generation::episode_drop::EpisodeDrop;
use crate::generation::generation_report::GenerationReport;
use crate::services::window_expansion_episode_compiler::WindowExpansionEpisodeCompiler;
use crate::services::window_expansion_oracle::WindowExpansionOracle;
use crate::use_cases::expand_session_transcript_use_case::ExpandSessionTranscriptUseCase;
use crate::use_cases::run_operator_session_multi_step_use_case::RunOperatorSessionMultiStepUseCase;

#[derive(Debug)]
pub struct GenerateWindowExpansionsUseCase {
    session: RunOperatorSessionMultiStepUseCase,
    task_family: TaskFamily,
}

impl GenerateWindowExpansionsUseCase {
    pub fn new(session: RunOperatorSessionMultiStepUseCase, task_family: TaskFamily) -> Self {
        Self {
            session,
            task_family,
        }
    }

    pub fn execute(
        &self,
        episodes: &[WindowExpansionEpisode],
    ) -> Result<GenerationReport, GenerateWindowExpansionsError> {
        let mut trajectories = Vec::new();
        let mut drops = Vec::new();
        let mut accepted_episodes = 0usize;

        for (index, episode) in episodes.iter().enumerate() {
            let session_id = OperatorSessionId::parse(format!("we:{index:04}")).map_err(|err| {
                GenerateWindowExpansionsError::SessionId {
                    index,
                    message: err.to_string(),
                }
            })?;
            let request = WindowExpansionEpisodeCompiler::compile(
                session_id,
                episode.about().clone(),
                episode.goal().clone(),
                &episode.spec(),
                episode.token_budget(),
            )
            .map_err(|err| GenerateWindowExpansionsError::Compile {
                index,
                message: err.to_string(),
            })?;

            let transcript = self.session.execute(&request).map_err(|err| {
                GenerateWindowExpansionsError::Session {
                    index,
                    message: err.to_string(),
                }
            })?;

            let outcome = WindowExpansionOracle::verify(&transcript);
            let final_state = transcript.final_visible_state();
            let conflict_blocking = final_state.coverage_deviation().conflict_blocking();
            // Gold is authoritative when present: keep the trajectory only if
            // every expected period entry was actually retrieved. With no gold,
            // fall back to the kernel's in-band coverage signal.
            let gold_covered = episode
                .expected_refs()
                .iter()
                .all(|memory_ref| final_state.knows_ref(memory_ref));
            let covered = if episode.has_gold() {
                gold_covered
            } else {
                outcome.is_covered()
            };

            if covered && !conflict_blocking {
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
                drops.push(EpisodeDrop::new(
                    episode.about().as_str().to_string(),
                    outcome,
                    conflict_blocking,
                    gold_covered,
                ));
            }
        }

        Ok(GenerationReport::new(
            trajectories,
            drops,
            accepted_episodes,
        ))
    }
}
