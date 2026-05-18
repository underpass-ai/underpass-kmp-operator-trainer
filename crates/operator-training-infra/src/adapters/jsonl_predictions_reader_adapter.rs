//! `PredictionsReader` adapter that wraps the
//! `operator-evaluation-infra::JsonlPredictionsReader`. The wrapping
//! is necessary because the application-layer port returns a
//! training-application `PredictionsReadError`, while the infra
//! crate returns its own evaluation-infra error type. This adapter
//! maps one onto the other so the training use cases stay decoupled
//! from `operator-evaluation-infra`.

use operator_evaluation_domain::prediction::step_keyed_prediction::StepKeyedPrediction;
use operator_evaluation_infra::adapters::jsonl_predictions_reader::JsonlPredictionsReader;
use operator_evaluation_infra::errors::predictions_read_error::PredictionsReadError as InfraError;
use operator_training_application::errors::predictions_read_error::PredictionsReadError;
use operator_training_application::ports::predictions_reader::PredictionsReader;

#[derive(Debug, Clone)]
pub struct JsonlPredictionsReaderAdapter {
    inner: JsonlPredictionsReader,
}

impl JsonlPredictionsReaderAdapter {
    pub fn new(source_path: impl Into<std::path::PathBuf>) -> Self {
        Self {
            inner: JsonlPredictionsReader::new(source_path),
        }
    }
}

impl PredictionsReader for JsonlPredictionsReaderAdapter {
    fn read(&self) -> Result<Vec<StepKeyedPrediction>, PredictionsReadError> {
        self.inner.read().map_err(translate_error)
    }
}

fn translate_error(err: InfraError) -> PredictionsReadError {
    match err {
        InfraError::SourceUnavailable { adapter, message } => {
            PredictionsReadError::SourceUnavailable { adapter, message }
        }
        InfraError::InvalidRow {
            adapter,
            line,
            message,
        } => PredictionsReadError::InvalidRow {
            adapter,
            line,
            message,
        },
        InfraError::ShapeViolation {
            adapter,
            line,
            message,
        } => PredictionsReadError::ShapeViolation {
            adapter,
            line,
            message,
        },
    }
}
