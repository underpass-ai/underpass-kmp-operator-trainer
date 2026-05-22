//! Human-readable stderr progress sink for paid realistic corpus runs.

use std::sync::Mutex;
use std::time::{Duration, Instant};

use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;
use operator_synthetic_application::error::corpus_event_sink_error::CorpusEventSinkError;
use operator_synthetic_application::ports::corpus_event_sink::CorpusEventSink;
use operator_synthetic_application::ports::scenario::Scenario;
use operator_synthetic_application::use_cases::drop_entry::DropEntry;
use operator_synthetic_application::use_cases::realistic_corpus_report::RealisticCorpusReport;
use serde_json::json;

#[derive(Debug)]
pub struct StderrProgressSink {
    every_n_rows: usize,
    every_duration: Duration,
    state: Mutex<ProgressState>,
}

impl Default for StderrProgressSink {
    fn default() -> Self {
        Self {
            every_n_rows: 25,
            every_duration: Duration::from_secs(30),
            state: Mutex::new(ProgressState::default()),
        }
    }
}

#[derive(Debug)]
struct ProgressState {
    last_emit: Instant,
    emitted_any_row: bool,
}

impl Default for ProgressState {
    fn default() -> Self {
        Self {
            last_emit: Instant::now(),
            emitted_any_row: false,
        }
    }
}

impl CorpusEventSink for StderrProgressSink {
    fn on_run_started(&self, total_scenarios: usize) -> Result<(), CorpusEventSinkError> {
        eprintln!(
            "{}",
            json!({
                "event": "realistic_corpus.run_started",
                "total_scenarios": total_scenarios,
            })
        );
        Ok(())
    }

    fn on_row_accepted(
        &self,
        index: usize,
        scenario: &Scenario,
        trajectory: &TrainingTrajectory,
    ) -> Result<(), CorpusEventSinkError> {
        if !self.should_emit(index)? {
            return Ok(());
        }
        eprintln!(
            "{}",
            json!({
                "event": "realistic_corpus.accepted",
                "index": index,
                "scenario_id": scenario.id().as_str(),
                "target": scenario.target().name(),
                "step_id": trajectory.step_id().as_str(),
            })
        );
        Ok(())
    }

    fn on_row_dropped(
        &self,
        index: usize,
        scenario: &Scenario,
        drop: &DropEntry,
    ) -> Result<(), CorpusEventSinkError> {
        eprintln!(
            "{}",
            json!({
                "event": "realistic_corpus.drop",
                "index": index,
                "scenario_id": scenario.id().as_str(),
                "target": scenario.target().name(),
                "reason_kind": drop.reason().kind().as_str(),
            })
        );
        Ok(())
    }

    fn on_run_finished(&self, report: &RealisticCorpusReport) -> Result<(), CorpusEventSinkError> {
        eprintln!(
            "{}",
            json!({
                "event": "realistic_corpus.run_finished",
                "total_scenarios": report.total_scenarios(),
                "accepted_count": report.accepted_count(),
                "dropped_count": report.dropped_count(),
                "drop_rate": report.drop_rate(),
                "gate_passed": report.gate_passed(),
            })
        );
        Ok(())
    }
}

impl StderrProgressSink {
    fn should_emit(&self, index: usize) -> Result<bool, CorpusEventSinkError> {
        let mut state = self.state.lock().map_err(|_| CorpusEventSinkError::Sink {
            adapter: "stderr_progress_sink",
            message: "progress state lock poisoned".to_string(),
        })?;
        let is_cadence = (index + 1).is_multiple_of(self.every_n_rows);
        let is_duration = state.last_emit.elapsed() >= self.every_duration;
        let should_emit = !state.emitted_any_row || is_cadence || is_duration;
        if should_emit {
            state.last_emit = Instant::now();
            state.emitted_any_row = true;
        }
        Ok(should_emit)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_first_row_and_then_every_configured_cadence() {
        let sink = StderrProgressSink::default();

        assert!(sink.should_emit(0).unwrap());
        assert!(!sink.should_emit(1).unwrap());
        assert!(sink.should_emit(24).unwrap());
    }
}
