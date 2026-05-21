//! Port: ask a teacher model for the next operator action.

use operator_shared_domain::action::operator_action::OperatorAction;
use operator_synthetic_domain::calibration::calibration_subject::CalibrationSubject;

use crate::error::teacher_policy_error::TeacherPolicyError;

pub trait TeacherPolicy: std::fmt::Debug + Send + Sync {
    fn decide(&self, subject: &CalibrationSubject) -> Result<OperatorAction, TeacherPolicyError>;
}

impl<T> TeacherPolicy for Box<T>
where
    T: TeacherPolicy + ?Sized,
{
    fn decide(&self, subject: &CalibrationSubject) -> Result<OperatorAction, TeacherPolicyError> {
        (**self).decide(subject)
    }
}
