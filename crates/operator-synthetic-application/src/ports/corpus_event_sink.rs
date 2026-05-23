//! Port for streaming realistic corpus production events while a run is active.

use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;

use crate::error::corpus_event_sink_error::CorpusEventSinkError;
use crate::ports::scenario::Scenario;
use crate::use_cases::drop_entry::DropEntry;
use crate::use_cases::realistic_corpus_report::RealisticCorpusReport;

pub trait CorpusEventSink: std::fmt::Debug + Send + Sync {
    fn on_run_started(&self, total_scenarios: usize) -> Result<(), CorpusEventSinkError>;

    fn on_row_accepted(
        &self,
        index: usize,
        scenario: &Scenario,
        trajectory: &TrainingTrajectory,
    ) -> Result<(), CorpusEventSinkError>;

    fn on_row_dropped(
        &self,
        index: usize,
        scenario: &Scenario,
        drop: &DropEntry,
    ) -> Result<(), CorpusEventSinkError>;

    fn on_run_finished(&self, report: &RealisticCorpusReport) -> Result<(), CorpusEventSinkError>;
}

impl<T> CorpusEventSink for Box<T>
where
    T: CorpusEventSink + ?Sized,
{
    fn on_run_started(&self, total_scenarios: usize) -> Result<(), CorpusEventSinkError> {
        (**self).on_run_started(total_scenarios)
    }

    fn on_row_accepted(
        &self,
        index: usize,
        scenario: &Scenario,
        trajectory: &TrainingTrajectory,
    ) -> Result<(), CorpusEventSinkError> {
        (**self).on_row_accepted(index, scenario, trajectory)
    }

    fn on_row_dropped(
        &self,
        index: usize,
        scenario: &Scenario,
        drop: &DropEntry,
    ) -> Result<(), CorpusEventSinkError> {
        (**self).on_row_dropped(index, scenario, drop)
    }

    fn on_run_finished(&self, report: &RealisticCorpusReport) -> Result<(), CorpusEventSinkError> {
        (**self).on_run_finished(report)
    }
}
