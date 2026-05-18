//! Typed return value of `ValidateTrainedRunUseCase::execute`.
//! Pairs the predictor's process-level outcome (where the files
//! landed, predictions count, failures count) with the model-level
//! `EvaluationReport` scored over the joined predictions. Callers
//! apply post-train readiness gates against either side or both.

use operator_evaluation_domain::report::evaluation_report::EvaluationReport;

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
}
