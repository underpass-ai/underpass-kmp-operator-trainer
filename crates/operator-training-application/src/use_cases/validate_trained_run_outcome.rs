//! Typed return value of `ValidateTrainedRunUseCase::execute`.
//! Pairs the predictor's process-level outcome (where the files
//! landed, predictions count, failures count) with the model-level
//! `EvaluationReport` scored over the joined predictions. Callers
//! apply post-train readiness gates against either side or both.

use operator_evaluation_domain::report::evaluation_report::EvaluationReport;
use operator_training_domain::readiness::pass_rate_percent::PassRatePercent;

use crate::ports::predictor_outcome::PredictorOutcome;

#[derive(Debug, Clone, PartialEq)]
pub struct ValidateTrainedRunOutcome {
    predictor_outcome: PredictorOutcome,
    evaluation_report: EvaluationReport,
}

impl ValidateTrainedRunOutcome {
    pub fn new(predictor_outcome: PredictorOutcome, evaluation_report: EvaluationReport) -> Self {
        Self {
            predictor_outcome,
            evaluation_report,
        }
    }

    pub fn predictor_outcome(&self) -> &PredictorOutcome {
        &self.predictor_outcome
    }

    pub fn evaluation_report(&self) -> &EvaluationReport {
        &self.evaluation_report
    }

    /// One-line verdict suitable for both logging and a downstream
    /// readiness gate: the predictor must have produced at least one
    /// prediction with zero declared failures, AND the evaluation
    /// report's exact-match rate must reach `min_pass_rate`. Callers
    /// that want richer logic (e.g., compound gates on multiple
    /// rates) read the report directly instead.
    pub fn is_passing(&self, min_pass_rate: PassRatePercent) -> bool {
        let predictor_clean =
            self.predictor_outcome.predictions() > 0 && self.predictor_outcome.failures() == 0;
        predictor_clean && self.evaluation_report.exact_match_rate() >= min_pass_rate.as_f64()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outcome_with(predictions: usize, failures: usize) -> PredictorOutcome {
        PredictorOutcome::new("/tmp/p", "/tmp/s", predictions, failures).unwrap()
    }

    #[test]
    fn passing_when_predictor_clean_and_rate_meets_threshold() {
        let v = ValidateTrainedRunOutcome::new(
            outcome_with(10, 0),
            EvaluationReport::from_outcomes(Vec::new()),
        );
        // Empty report -> rate 0.0; threshold 0.0 -> still passes.
        assert!(v.is_passing(PassRatePercent::parse(0.0).unwrap()));
    }

    #[test]
    fn failing_when_predictor_has_failures() {
        let v = ValidateTrainedRunOutcome::new(
            outcome_with(10, 1),
            EvaluationReport::from_outcomes(Vec::new()),
        );
        assert!(!v.is_passing(PassRatePercent::parse(0.0).unwrap()));
    }

    #[test]
    fn failing_when_predictor_produced_nothing() {
        let v = ValidateTrainedRunOutcome::new(
            outcome_with(0, 0),
            EvaluationReport::from_outcomes(Vec::new()),
        );
        assert!(!v.is_passing(PassRatePercent::parse(0.0).unwrap()));
    }

    #[test]
    fn failing_when_rate_below_threshold() {
        let v = ValidateTrainedRunOutcome::new(
            outcome_with(10, 0),
            EvaluationReport::from_outcomes(Vec::new()),
        );
        // Empty report -> rate 0.0; threshold 0.9 -> fails.
        assert!(!v.is_passing(PassRatePercent::parse(0.9).unwrap()));
    }
}
