//! JSONL source for runtime teacher calibration cases.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use operator_synthetic_application::error::calibration_episode_source_error::CalibrationEpisodeSourceError;
use operator_synthetic_application::ports::calibration_episode_source::CalibrationEpisodeSource;
use operator_synthetic_domain::calibration::calibration_case::CalibrationCase;

use crate::dto::calibration_case_dto::CalibrationCaseDto;
use crate::mappers::calibration_case_mapper::CalibrationCaseMapper;

const ADAPTER: &str = "jsonl_calibration_episode_source";

#[derive(Debug, Clone)]
pub struct JsonlCalibrationEpisodeSource {
    path: PathBuf,
    limit: Option<usize>,
}

impl JsonlCalibrationEpisodeSource {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            limit: None,
        }
    }

    #[must_use]
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = (limit > 0).then_some(limit);
        self
    }
}

impl CalibrationEpisodeSource for JsonlCalibrationEpisodeSource {
    fn read(&self) -> Result<Vec<CalibrationCase>, CalibrationEpisodeSourceError> {
        let file = File::open(&self.path).map_err(|err| {
            CalibrationEpisodeSourceError::SourceUnavailable {
                adapter: ADAPTER,
                message: format!("open {}: {err}", self.path.display()),
            }
        })?;
        let reader = BufReader::new(file);
        let mut cases = Vec::new();
        for (zero_based, line) in reader.lines().enumerate() {
            let line_number = zero_based + 1;
            let line = line.map_err(|err| CalibrationEpisodeSourceError::InvalidRow {
                adapter: ADAPTER,
                line: line_number,
                message: err.to_string(),
            })?;
            if line.trim().is_empty() {
                continue;
            }
            let dto: CalibrationCaseDto = serde_json::from_str(&line).map_err(|err| {
                CalibrationEpisodeSourceError::InvalidRow {
                    adapter: ADAPTER,
                    line: line_number,
                    message: err.to_string(),
                }
            })?;
            let case = CalibrationCaseMapper::to_domain(&dto).map_err(|err| {
                CalibrationEpisodeSourceError::InvalidRow {
                    adapter: ADAPTER,
                    line: line_number,
                    message: err.to_string(),
                }
            })?;
            cases.push(case);
        }
        if cases.is_empty() {
            return Err(CalibrationEpisodeSourceError::EmptySource { adapter: ADAPTER });
        }
        if let Some(limit) = self.limit {
            cases.truncate(limit);
        }
        Ok(cases)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn reads_well_formed_jsonl() {
        let path = temp_path("well-formed.jsonl");
        fs::write(&path, fixture_line()).unwrap();
        let rows = JsonlCalibrationEpisodeSource::new(&path).read().unwrap();
        assert_eq!(rows.len(), 1);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn errors_on_malformed_line_with_line_number() {
        let path = temp_path("malformed.jsonl");
        fs::write(&path, "{bad}\n").unwrap();
        let err = JsonlCalibrationEpisodeSource::new(&path)
            .read()
            .unwrap_err();
        assert!(matches!(
            err,
            CalibrationEpisodeSourceError::InvalidRow { line: 1, .. }
        ));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn errors_on_empty_file() {
        let path = temp_path("empty.jsonl");
        fs::write(&path, "\n").unwrap();
        let err = JsonlCalibrationEpisodeSource::new(&path)
            .read()
            .unwrap_err();
        assert!(matches!(
            err,
            CalibrationEpisodeSourceError::EmptySource { .. }
        ));
        let _ = fs::remove_file(path);
    }

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "operator-calibration-source-{}-{name}",
            std::process::id()
        ))
    }

    fn fixture_line() -> String {
        r#"{"case_id":"calib:inspect","domain_theme":"technical_incident","category":"happy","subject":{"about":"about:incident","mode":"read","task_family":"read.inspect","goal":"Inspect visible evidence.","allowed_tools":["kernel_wake","kernel_ask","kernel_near","kernel_goto","kernel_rewind","kernel_forward","kernel_trace","kernel_inspect"],"visible_state":{"known_refs":["node:target"],"known_dimensions":[],"budget":{"calls_remaining":3,"tokens_remaining":1024}}},"accepted_actions":[{"kind":"tool_call","tool":"kernel_inspect","arguments":{"target":"node:target"}}],"expected_action_rationale":"The ref is already visible."}"#.to_string()
    }
}
