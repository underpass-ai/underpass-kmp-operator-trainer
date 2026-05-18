//! Typed value object returned by `Predictor::predict`. Captures the
//! filesystem paths the predictor wrote, plus the predictions and
//! failures counts the predictor reports in its summary.json so the
//! caller does not have to re-read the file just to know whether
//! anything came out.

use operator_shared_domain::error::domain_result::DomainResult;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PredictorOutcome {
    predictions_path: NonEmptyString,
    summary_path: NonEmptyString,
    predictions: usize,
    failures: usize,
}

impl PredictorOutcome {
    pub fn new(
        predictions_path: impl Into<String>,
        summary_path: impl Into<String>,
        predictions: usize,
        failures: usize,
    ) -> DomainResult<Self> {
        Ok(Self {
            predictions_path: NonEmptyString::parse(
                predictions_path,
                "predictor_outcome.predictions_path",
            )?,
            summary_path: NonEmptyString::parse(summary_path, "predictor_outcome.summary_path")?,
            predictions,
            failures,
        })
    }

    pub fn predictions_path(&self) -> &str {
        self.predictions_path.as_str()
    }

    pub fn summary_path(&self) -> &str {
        self.summary_path.as_str()
    }

    pub fn predictions(&self) -> usize {
        self.predictions
    }

    pub fn failures(&self) -> usize {
        self.failures
    }
}
