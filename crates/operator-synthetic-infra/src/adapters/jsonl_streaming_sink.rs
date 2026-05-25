//! Streams accepted and dropped realistic corpus rows to partial JSONL files.

use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::Mutex;

use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;
use operator_shared_infra::mappers::training_trajectory_mapper::TrainingTrajectoryMapper;
use operator_synthetic_application::error::corpus_event_sink_error::CorpusEventSinkError;
use operator_synthetic_application::ports::corpus_event_sink::CorpusEventSink;
use operator_synthetic_application::ports::scenario::Scenario;
use operator_synthetic_application::use_cases::drop_entry::DropEntry;
use operator_synthetic_application::use_cases::realistic_corpus_report::RealisticCorpusReport;

use crate::mappers::realistic_corpus_report_mapper::RealisticCorpusReportMapper;

const ADAPTER: &str = "jsonl_streaming_sink";
pub const TRAJECTORIES_PARTIAL_FILE: &str = "trajectories.partial.jsonl";
pub const DROPPED_PARTIAL_FILE: &str = "dropped.partial.jsonl";

#[derive(Debug)]
pub struct JsonlStreamingSink {
    trajectories: Mutex<BufWriter<File>>,
    dropped: Mutex<BufWriter<File>>,
}

impl JsonlStreamingSink {
    pub fn new(output_dir: &Path) -> Result<Self, CorpusEventSinkError> {
        fs::create_dir_all(output_dir).map_err(|err| CorpusEventSinkError::Sink {
            adapter: ADAPTER,
            message: format!("create {}: {err}", output_dir.display()),
        })?;
        Ok(Self {
            trajectories: Mutex::new(writer(output_dir.join(TRAJECTORIES_PARTIAL_FILE))?),
            dropped: Mutex::new(writer(output_dir.join(DROPPED_PARTIAL_FILE))?),
        })
    }
}

impl CorpusEventSink for JsonlStreamingSink {
    fn on_run_started(&self, _total_scenarios: usize) -> Result<(), CorpusEventSinkError> {
        Ok(())
    }

    fn on_row_accepted(
        &self,
        _index: usize,
        _scenario: &Scenario,
        trajectory: &TrainingTrajectory,
    ) -> Result<(), CorpusEventSinkError> {
        let dto = TrainingTrajectoryMapper::to_dto(trajectory).map_err(|err| {
            CorpusEventSinkError::Sink {
                adapter: ADAPTER,
                message: format!("map trajectory: {err}"),
            }
        })?;
        write_jsonl(&self.trajectories, &dto)
    }

    fn on_row_dropped(
        &self,
        _index: usize,
        _scenario: &Scenario,
        drop: &DropEntry,
    ) -> Result<(), CorpusEventSinkError> {
        let dto = RealisticCorpusReportMapper::drop_to_dto(drop).map_err(|err| {
            CorpusEventSinkError::Sink {
                adapter: ADAPTER,
                message: format!("map drop: {err}"),
            }
        })?;
        write_jsonl(&self.dropped, &dto)
    }

    fn on_run_finished(&self, _report: &RealisticCorpusReport) -> Result<(), CorpusEventSinkError> {
        sync_writer(&self.trajectories)?;
        sync_writer(&self.dropped)
    }
}

fn writer(path: impl AsRef<Path>) -> Result<BufWriter<File>, CorpusEventSinkError> {
    let path = path.as_ref();
    let file = File::create(path).map_err(|err| CorpusEventSinkError::Sink {
        adapter: ADAPTER,
        message: format!("create {}: {err}", path.display()),
    })?;
    Ok(BufWriter::new(file))
}

fn write_jsonl<T: serde::Serialize>(
    writer: &Mutex<BufWriter<File>>,
    dto: &T,
) -> Result<(), CorpusEventSinkError> {
    let mut writer = writer.lock().map_err(|_| CorpusEventSinkError::Sink {
        adapter: ADAPTER,
        message: "event file lock poisoned".to_string(),
    })?;
    serde_json::to_writer(&mut *writer, dto).map_err(|err| CorpusEventSinkError::Sink {
        adapter: ADAPTER,
        message: format!("serialize row: {err}"),
    })?;
    writeln!(&mut *writer).map_err(|err| CorpusEventSinkError::Sink {
        adapter: ADAPTER,
        message: format!("write row newline: {err}"),
    })?;
    writer.flush().map_err(|err| CorpusEventSinkError::Sink {
        adapter: ADAPTER,
        message: format!("flush row: {err}"),
    })
}

fn sync_writer(writer: &Mutex<BufWriter<File>>) -> Result<(), CorpusEventSinkError> {
    let mut writer = writer.lock().map_err(|_| CorpusEventSinkError::Sink {
        adapter: ADAPTER,
        message: "event file lock poisoned".to_string(),
    })?;
    writer.flush().map_err(|err| CorpusEventSinkError::Sink {
        adapter: ADAPTER,
        message: format!("flush before sync: {err}"),
    })?;
    writer
        .get_ref()
        .sync_all()
        .map_err(|err| CorpusEventSinkError::Sink {
            adapter: ADAPTER,
            message: format!("sync file: {err}"),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::ids::about_id::AboutId;
    use operator_shared_domain::mode::allowed_tools::AllowedTools;
    use operator_shared_domain::mode::operator_mode::OperatorMode;
    use operator_shared_domain::value_objects::subject_hash::SubjectHash;
    use operator_shared_domain::value_objects::task_family::TaskFamily;
    use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
    use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
    use operator_shared_domain::visible_state::visible_state::VisibleState;
    use operator_synthetic_application::ports::scenario_id::ScenarioId;
    use operator_synthetic_application::use_cases::drop_reason::DropReason;
    use operator_synthetic_domain::calibration::calibration_subject::CalibrationSubject;
    use operator_synthetic_domain::capability::kmp_mcp_capability::KmpMcpCapability;
    use operator_synthetic_domain::case::synthetic_acceptance_criteria::SyntheticAcceptanceCriteria;
    use operator_synthetic_domain::case::synthetic_generation_target::SyntheticGenerationTarget;

    #[test]
    fn creates_partial_files_on_construction() {
        let root = std::env::temp_dir().join(format!(
            "operator-jsonl-streaming-sink-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);

        JsonlStreamingSink::new(&root).unwrap();

        assert!(root.join(TRAJECTORIES_PARTIAL_FILE).is_file());
        assert!(root.join(DROPPED_PARTIAL_FILE).is_file());
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn writes_dropped_rows_immediately() {
        let root = std::env::temp_dir().join(format!(
            "operator-jsonl-streaming-sink-drop-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let sink = JsonlStreamingSink::new(&root).unwrap();
        let scenario = scenario();
        let drop = DropEntry::new(
            scenario.id().clone(),
            scenario.target(),
            DropReason::TeacherError {
                message: "shape failed".to_string(),
            },
            None,
            subject_hash(),
            None,
        );

        sink.on_row_dropped(0, &scenario, &drop).unwrap();

        let contents = fs::read_to_string(root.join(DROPPED_PARTIAL_FILE)).unwrap();
        assert!(contents.contains("\"teacher_error\""));
        assert!(contents.contains("\"scenario:inspect\""));
        assert!(contents.contains("\"predicted_action\":null"));
        assert!(contents.contains("\"subject_hash\""));
        assert!(contents.contains("\"teacher_finish_reason\":null"));
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn writes_truncation_raw_content_tail_to_dropped_jsonl() {
        let root = std::env::temp_dir().join(format!(
            "operator-jsonl-streaming-sink-truncation-drop-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let sink = JsonlStreamingSink::new(&root).unwrap();
        let scenario = scenario();
        let drop = DropEntry::new(
            scenario.id().clone(),
            scenario.target(),
            DropReason::TeacherTruncation {
                finish_reason:
                    operator_shared_domain::value_objects::finish_reason::FinishReason::Length,
                content_len: 8373,
                raw_content_tail: "\"arguments\":{\"memory\":".to_string(),
                request_bytes: 15830,
                response_bytes: 13276,
                elapsed_ms: 112_606,
                request_id: Some("req_test".to_string()),
            },
            None,
            subject_hash(),
            Some(operator_shared_domain::value_objects::finish_reason::FinishReason::Length),
        );

        sink.on_row_dropped(0, &scenario, &drop).unwrap();

        let contents = fs::read_to_string(root.join(DROPPED_PARTIAL_FILE)).unwrap();
        assert!(contents.contains("\"reason\":\"teacher_truncation\""));
        assert!(contents.contains("\"teacher_finish_reason\":\"length\""));
        assert!(contents.contains("\"raw_content_tail\":\"\\\"arguments\\\":{\\\"memory\\\":\""));
        let _ = fs::remove_dir_all(root);
    }

    fn scenario() -> Scenario {
        Scenario::new(
            ScenarioId::parse("scenario:inspect").unwrap(),
            SyntheticGenerationTarget::from(KmpMcpCapability::Inspect),
            CalibrationSubject::new(
                AboutId::parse("about:incident").unwrap(),
                OperatorMode::Read,
                TaskFamily::parse("realistic.kernel_inspect").unwrap(),
                TrajectoryGoal::parse("Inspect visible evidence.").unwrap(),
                AllowedTools::for_mode(OperatorMode::Read),
                VisibleState::assemble([], [], None, BudgetSnapshot::bounded(3, 1024)),
                None,
            )
            .unwrap(),
            SyntheticAcceptanceCriteria::permissive(),
            subject_hash(),
        )
    }

    fn subject_hash() -> SubjectHash {
        SubjectHash::parse("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
            .unwrap()
    }
}
