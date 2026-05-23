//! `PredictionsReader` adapter that wraps the
//! `operator-evaluation-infra::JsonlPredictionsReader`. The wrapping
//! is necessary because the application-layer port returns a
//! training-application `PredictionsReadError`, while the infra
//! crate returns its own evaluation-infra error type. This adapter
//! maps one onto the other so the training use cases stay decoupled
//! from `operator-evaluation-infra`.
//!
//! Why a named adapter struct and not a `From` impl: every adapter
//! in this crate (`ProcessTrainerInvoker`, `ProcessPredictorInvoker`,
//! `CompositePolicyEvaluator`, the two filesystem writers) is a
//! named struct that implements its port trait. Keeping
//! `JsonlPredictionsReaderAdapter` in the same shape makes the
//! adapter inventory consistent at a glance and gives the wrapper a
//! place to grow when the wire shape inevitably needs to diverge
//! from the application port (e.g., for an alternate JSONL dialect
//! emitted by a future predictor).

use operator_evaluation_domain::prediction::predictions_read_outcome::PredictionsReadOutcome;
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
    fn read(&self) -> Result<PredictionsReadOutcome, PredictionsReadError> {
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
