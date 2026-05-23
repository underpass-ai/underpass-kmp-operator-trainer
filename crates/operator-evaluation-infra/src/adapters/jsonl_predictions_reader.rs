//! Filesystem reader for `predictions.jsonl` as produced by the
//! Python `predict_operator_sft.py` script in `scripts/operator/`.
//! Each line is a `{step_id, action}` object; the reader emits a
//! `Vec<StepKeyedPrediction>` after mapping the wire action through
//! the shared `OperatorActionMapper`.
//!
//! Empty / whitespace-only lines are skipped silently — matches the
//! script's batch flushing semantics.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use operator_evaluation_domain::prediction::predictions_read_outcome::PredictionsReadOutcome;
use operator_evaluation_domain::prediction::shape_violation_record::ShapeViolationRecord;
use operator_evaluation_domain::prediction::step_keyed_prediction::StepKeyedPrediction;
use operator_shared_domain::ids::step_id::StepId;
use operator_shared_infra::mappers::operator_action_mapper::OperatorActionMapper;

use crate::dto::prediction_row_dto::PredictionRowDto;
use crate::errors::predictions_read_error::PredictionsReadError;

const ADAPTER: &str = "jsonl_predictions_reader";

#[derive(Debug, Clone)]
pub struct JsonlPredictionsReader {
    source_path: PathBuf,
}

impl JsonlPredictionsReader {
    pub fn new(source_path: impl Into<PathBuf>) -> Self {
        Self {
            source_path: source_path.into(),
        }
    }

    pub fn source_path(&self) -> &Path {
        &self.source_path
    }

    pub fn read(&self) -> Result<PredictionsReadOutcome, PredictionsReadError> {
        let file = File::open(&self.source_path).map_err(|err| {
            PredictionsReadError::SourceUnavailable {
                adapter: ADAPTER,
                message: format!("open {}: {err}", self.source_path.display()),
            }
        })?;
        let reader = BufReader::new(file);
        let mut parsed = Vec::new();
        let mut shape_violations = Vec::new();
        for (index, line) in reader.lines().enumerate() {
            let line_number = index + 1;
            let line = line.map_err(|err| PredictionsReadError::SourceUnavailable {
                adapter: ADAPTER,
                message: format!("read line {line_number}: {err}"),
            })?;
            if line.trim().is_empty() {
                continue;
            }
            let dto: PredictionRowDto =
                serde_json::from_str(&line).map_err(|err| PredictionsReadError::InvalidRow {
                    adapter: ADAPTER,
                    line: line_number,
                    message: err.to_string(),
                })?;
            let step_id = StepId::parse(dto.step_id.clone()).map_err(|err| {
                PredictionsReadError::ShapeViolation {
                    adapter: ADAPTER,
                    line: line_number,
                    message: format!("step_id `{}`: {err}", dto.step_id),
                }
            })?;
            match OperatorActionMapper::to_domain(&dto.action) {
                Ok(action) => {
                    parsed.push(StepKeyedPrediction::new(step_id, action));
                }
                Err(err) => {
                    let message = format!("action: {err}");
                    let record = ShapeViolationRecord::new(line_number, Some(step_id), message)
                        .map_err(|err| PredictionsReadError::ShapeViolation {
                            adapter: ADAPTER,
                            line: line_number,
                            message: err.to_string(),
                        })?;
                    shape_violations.push(record);
                }
            }
        }
        Ok(PredictionsReadOutcome::new(parsed, shape_violations))
    }
}
