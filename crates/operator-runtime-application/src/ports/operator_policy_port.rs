use operator_shared_domain::action::operator_action::OperatorAction;
use operator_synthetic_domain::calibration::calibration_subject::CalibrationSubject;

use crate::errors::operator_policy_error::OperatorPolicyError;

pub trait OperatorPolicy: std::fmt::Debug + Send + Sync {
    fn predict(&self, subject: &CalibrationSubject) -> Result<OperatorAction, OperatorPolicyError>;
}
