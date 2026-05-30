//! Drive the multi-step runtime loop with a synthetic teacher policy.
//!
//! The runtime loop predicts each step through the [`OperatorPolicy`] port; the
//! synthetic side already has calibrated teachers behind the [`TeacherPolicy`]
//! port (e.g. the OpenAI-compatible adapter). This adapter bridges the two so
//! the teacher demonstrates the window-expansion policy at every step of a real
//! session: its demonstrated actions become the trajectories a student later
//! learns from. It only forwards `decide -> predict` and translates the error
//! taxonomy; it adds no policy of its own.

use operator_runtime_application::errors::operator_policy_error::OperatorPolicyError;
use operator_runtime_application::ports::operator_policy_port::OperatorPolicy;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_synthetic_application::error::teacher_policy_error::TeacherPolicyError;
use operator_synthetic_application::ports::teacher_policy::TeacherPolicy;
use operator_synthetic_domain::calibration::calibration_subject::CalibrationSubject;
use operator_synthetic_domain::calibration::teacher_decision::TeacherDecision;

#[derive(Debug)]
pub struct TeacherBackedOperatorPolicy<T: TeacherPolicy> {
    teacher: T,
}

impl<T: TeacherPolicy> TeacherBackedOperatorPolicy<T> {
    pub fn new(teacher: T) -> Self {
        Self { teacher }
    }
}

impl<T: TeacherPolicy> OperatorPolicy for TeacherBackedOperatorPolicy<T> {
    fn predict(&self, subject: &CalibrationSubject) -> Result<OperatorAction, OperatorPolicyError> {
        self.teacher
            .decide(subject)
            .map(TeacherDecision::into_action)
            .map_err(map_error)
    }
}

fn map_error(error: TeacherPolicyError) -> OperatorPolicyError {
    match error {
        TeacherPolicyError::Transport { adapter, message } => OperatorPolicyError::Transport {
            message: format!("{adapter}: {message}"),
        },
        TeacherPolicyError::ApiError {
            adapter,
            code,
            message,
        } => OperatorPolicyError::Transport {
            message: match code {
                Some(code) => format!("{adapter} api error {code}: {message}"),
                None => format!("{adapter} api error: {message}"),
            },
        },
        TeacherPolicyError::Protocol { adapter, message } => OperatorPolicyError::Protocol {
            message: format!("{adapter}: {message}"),
        },
        TeacherPolicyError::Shape {
            adapter, message, ..
        } => OperatorPolicyError::Shape {
            message: format!("{adapter}: {message}"),
        },
        TeacherPolicyError::TruncatedResponse {
            adapter,
            finish_reason,
            ..
        } => OperatorPolicyError::Protocol {
            message: format!("{adapter} returned non-stop finish reason: {finish_reason}"),
        },
    }
}
