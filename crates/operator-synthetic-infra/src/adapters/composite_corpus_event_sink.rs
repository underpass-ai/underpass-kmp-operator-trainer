//! Multicasts realistic corpus events to several sinks.

use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;
use operator_synthetic_application::error::corpus_event_sink_error::CorpusEventSinkError;
use operator_synthetic_application::ports::corpus_event_sink::CorpusEventSink;
use operator_synthetic_application::ports::scenario::Scenario;
use operator_synthetic_application::use_cases::drop_entry::DropEntry;
use operator_synthetic_application::use_cases::realistic_corpus_report::RealisticCorpusReport;

#[derive(Debug, Default)]
pub struct CompositeCorpusEventSink {
    sinks: Vec<Box<dyn CorpusEventSink>>,
}

impl CompositeCorpusEventSink {
    pub fn new(sinks: Vec<Box<dyn CorpusEventSink>>) -> Self {
        Self { sinks }
    }
}

impl CorpusEventSink for CompositeCorpusEventSink {
    fn on_run_started(&self, total_scenarios: usize) -> Result<(), CorpusEventSinkError> {
        for sink in &self.sinks {
            sink.on_run_started(total_scenarios)?;
        }
        Ok(())
    }

    fn on_row_accepted(
        &self,
        index: usize,
        scenario: &Scenario,
        trajectory: &TrainingTrajectory,
    ) -> Result<(), CorpusEventSinkError> {
        for sink in &self.sinks {
            sink.on_row_accepted(index, scenario, trajectory)?;
        }
        Ok(())
    }

    fn on_row_dropped(
        &self,
        index: usize,
        scenario: &Scenario,
        drop: &DropEntry,
    ) -> Result<(), CorpusEventSinkError> {
        for sink in &self.sinks {
            sink.on_row_dropped(index, scenario, drop)?;
        }
        Ok(())
    }

    fn on_run_finished(&self, report: &RealisticCorpusReport) -> Result<(), CorpusEventSinkError> {
        for sink in &self.sinks {
            sink.on_run_finished(report)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct CountingSink {
        events: Arc<Mutex<Vec<&'static str>>>,
    }

    impl CorpusEventSink for CountingSink {
        fn on_run_started(&self, _total_scenarios: usize) -> Result<(), CorpusEventSinkError> {
            self.events.lock().unwrap().push("started");
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

        fn on_run_finished(
            &self,
            _report: &RealisticCorpusReport,
        ) -> Result<(), CorpusEventSinkError> {
            Ok(())
        }
    }

    #[test]
    fn delegates_started_event_to_all_wrapped_sinks() {
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink = CompositeCorpusEventSink::new(vec![
            Box::new(CountingSink {
                events: events.clone(),
            }),
            Box::new(CountingSink {
                events: events.clone(),
            }),
        ]);

        sink.on_run_started(0).unwrap();

        assert_eq!(*events.lock().unwrap(), vec!["started", "started"]);
    }
}
