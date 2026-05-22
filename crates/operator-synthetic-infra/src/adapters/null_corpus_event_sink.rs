//! No-op corpus event sink for tests and non-observed runs.

use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;
use operator_synthetic_application::error::corpus_event_sink_error::CorpusEventSinkError;
use operator_synthetic_application::ports::corpus_event_sink::CorpusEventSink;
use operator_synthetic_application::ports::scenario::Scenario;
use operator_synthetic_application::use_cases::drop_entry::DropEntry;
use operator_synthetic_application::use_cases::realistic_corpus_report::RealisticCorpusReport;

#[derive(Debug, Default)]
pub struct NullCorpusEventSink;

impl CorpusEventSink for NullCorpusEventSink {
    fn on_run_started(&self, _total_scenarios: usize) -> Result<(), CorpusEventSinkError> {
        Ok(())
    }

    fn on_row_accepted(
        &self,
        _index: usize,
        _scenario: &Scenario,
        _trajectory: &TrainingTrajectory,
    ) -> Result<(), CorpusEventSinkError> {
        Ok(())
    }

    fn on_row_dropped(
        &self,
        _index: usize,
        _scenario: &Scenario,
        _drop: &DropEntry,
    ) -> Result<(), CorpusEventSinkError> {
        Ok(())
    }

    fn on_run_finished(&self, _report: &RealisticCorpusReport) -> Result<(), CorpusEventSinkError> {
        Ok(())
    }
}
