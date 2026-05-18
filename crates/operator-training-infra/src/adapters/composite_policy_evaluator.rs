//! `PolicyEvaluator` adapter that wraps
//! `operator-evaluation-application::EvaluateOperatorPolicyUseCase`.
//! Keeps the training-application layer decoupled from
//! `operator-evaluation-application` while still letting training use
//! cases score predictions against the action contract.

use operator_evaluation_application::use_cases::evaluate_operator_policy_use_case::EvaluateOperatorPolicyUseCase;
use operator_evaluation_domain::prediction::evaluation_pair::EvaluationPair;
use operator_evaluation_domain::report::evaluation_report::EvaluationReport;
use operator_shared_domain::contract::action_contract_validator::ActionContractValidator;
use operator_training_application::errors::policy_evaluator_error::PolicyEvaluatorError;
use operator_training_application::ports::policy_evaluator::PolicyEvaluator;

const ADAPTER: &str = "composite_policy_evaluator";

#[derive(Debug)]
pub struct CompositePolicyEvaluator<V: ActionContractValidator + std::fmt::Debug> {
    inner: EvaluateOperatorPolicyUseCase<V>,
}

impl<V: ActionContractValidator + std::fmt::Debug> CompositePolicyEvaluator<V> {
    pub fn new(validator: V) -> Self {
        Self {
            inner: EvaluateOperatorPolicyUseCase::new(validator),
        }
    }
}

impl<V: ActionContractValidator + std::fmt::Debug + Send + Sync> PolicyEvaluator
    for CompositePolicyEvaluator<V>
{
    fn evaluate(&self, pairs: &[EvaluationPair]) -> Result<EvaluationReport, PolicyEvaluatorError> {
        self.inner
            .execute(pairs)
            .map_err(|err| PolicyEvaluatorError::DomainFailure {
                adapter: ADAPTER,
                message: err.to_string(),
            })
    }
}
