//! JSONL source for externally authored realistic corpus scenarios.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use operator_synthetic_application::error::scenario_source_error::ScenarioSourceError;
use operator_synthetic_application::ports::scenario::Scenario;
use operator_synthetic_application::ports::scenario_source::ScenarioSource;

use crate::dto::scenario_dto::ScenarioDto;
use crate::mappers::scenario_mapper::ScenarioMapper;

const ADAPTER: &str = "jsonl_scenario_source";

#[derive(Debug, Clone)]
pub struct JsonlScenarioSource {
    path: PathBuf,
    limit: Option<usize>,
}

impl JsonlScenarioSource {
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

impl ScenarioSource for JsonlScenarioSource {
    fn read(&self) -> Result<Vec<Scenario>, ScenarioSourceError> {
        let file =
            File::open(&self.path).map_err(|err| ScenarioSourceError::SourceUnavailable {
                adapter: ADAPTER,
                message: format!("open {}: {err}", self.path.display()),
            })?;
        let reader = BufReader::new(file);
        let mut scenarios = Vec::new();
        for (zero_based, line) in reader.lines().enumerate() {
            let line_number = zero_based + 1;
            let line = line.map_err(|err| ScenarioSourceError::InvalidRow {
                adapter: ADAPTER,
                line: line_number,
                message: err.to_string(),
            })?;
            if line.trim().is_empty() {
                continue;
            }
            let dto: ScenarioDto =
                serde_json::from_str(&line).map_err(|err| ScenarioSourceError::InvalidRow {
                    adapter: ADAPTER,
                    line: line_number,
                    message: err.to_string(),
                })?;
            let scenario = ScenarioMapper::to_application(&dto).map_err(|err| {
                ScenarioSourceError::InvalidRow {
                    adapter: ADAPTER,
                    line: line_number,
                    message: err.to_string(),
                }
            })?;
            scenarios.push(scenario);
        }
        if scenarios.is_empty() {
            return Err(ScenarioSourceError::EmptySource { adapter: ADAPTER });
        }
        if let Some(limit) = self.limit {
            scenarios.truncate(limit);
        }
        Ok(scenarios)
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
        let rows = JsonlScenarioSource::new(&path).read().unwrap();
        assert_eq!(rows.len(), 1);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn errors_on_malformed_line_with_line_number() {
        let path = temp_path("malformed.jsonl");
        fs::write(&path, "{bad}\n").unwrap();
        let err = JsonlScenarioSource::new(&path).read().unwrap_err();
        assert!(matches!(
            err,
            ScenarioSourceError::InvalidRow { line: 1, .. }
        ));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn errors_on_empty_file() {
        let path = temp_path("empty.jsonl");
        fs::write(&path, "\n").unwrap();
        let err = JsonlScenarioSource::new(&path).read().unwrap_err();
        assert!(matches!(err, ScenarioSourceError::EmptySource { .. }));
        let _ = fs::remove_file(path);
    }

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "operator-scenario-source-{}-{name}",
            std::process::id()
        ))
    }

    fn fixture_line() -> String {
        r#"{"scenario_id":"scenario:inspect","target":"kernel_inspect","subject":{"about":"about:incident","mode":"read","task_family":"realistic.inspect","goal":"Inspect visible evidence.","allowed_tools":["kernel_wake","kernel_ask","kernel_near","kernel_goto","kernel_rewind","kernel_forward","kernel_trace","kernel_inspect"],"visible_state":{"known_refs":["node:target"],"known_dimensions":[],"budget":{"calls_remaining":3,"tokens_remaining":1024}}}}"#.to_string()
    }
}
