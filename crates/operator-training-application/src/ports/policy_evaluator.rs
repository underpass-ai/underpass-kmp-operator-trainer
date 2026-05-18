//! Port: score a slice of `EvaluationPair` values against the action
//! contract and produce an `EvaluationReport`. Keeps the training
//! application crate clean of any dependency on
//! `operator-evaluation-application`; the adapter in
//! `operator-training-infra` wraps `EvaluateOperatorPolicyUseCase`.

use operator_evaluation_domain::prediction::evaluation_pair::EvaluationPair;
use operator_evaluation_domain::report::evaluation_report::EvaluationReport;

use crate::errors::policy_evaluator_error::PolicyEvaluatorError;

pub trait PolicyEvaluator: std::fmt::Debug + Send + Sync {
    fn evaluate(&self, pairs: &[EvaluationPair]) -> Result<EvaluationReport, PolicyEvaluatorError>;
}
