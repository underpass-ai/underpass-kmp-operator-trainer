//! Evaluation result for one teacher calibration case.

use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;
use operator_synthetic_domain::calibration::calibration_capability::CalibrationCapability;
use operator_synthetic_domain::calibration::calibration_case_category::CalibrationCaseCategory;
use operator_synthetic_domain::calibration::calibration_case_id::CalibrationCaseId;
use operator_synthetic_domain::calibration::expected_action_rationale::ExpectedActionRationale;

use crate::use_cases::teacher_calibration_prediction_outcome::TeacherCalibrationPredictionOutcome;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeacherCalibrationCaseResult {
    case_id: CalibrationCaseId,
    capability: CalibrationCapability,
    category: CalibrationCaseCategory,
    prediction_outcome: TeacherCalibrationPredictionOutcome,
    shape_status: ShapeStatus,
    expected_action_rationale: Option<ExpectedActionRationale>,
    failure_message: Option<NonEmptyString>,
    predicted_action: Option<OperatorAction>,
    accepted_actions: Vec<OperatorAction>,
}

impl TeacherCalibrationCaseResult {
    pub fn prediction(
        case_id: CalibrationCaseId,
        capability: CalibrationCapability,
        category: CalibrationCaseCategory,
        prediction_outcome: TeacherCalibrationPredictionOutcome,
    ) -> Self {
        Self {
            case_id,
            capability,
            category,
            prediction_outcome,
            shape_status: ShapeStatus::Valid,
            expected_action_rationale: None,
            failure_message: None,
            predicted_action: None,
            accepted_actions: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_failure_debug(
        mut self,
        expected_action_rationale: ExpectedActionRationale,
        predicted_action: OperatorAction,
        accepted_actions: Vec<OperatorAction>,
    ) -> Self {
        self.expected_action_rationale = Some(expected_action_rationale);
        self.predicted_action = Some(predicted_action);
        self.accepted_actions = accepted_actions;
        self
    }

    pub fn shape_failure(
        case_id: CalibrationCaseId,
        capability: CalibrationCapability,
        category: CalibrationCaseCategory,
        expected_action_rationale: Option<ExpectedActionRationale>,
        failure_message: Option<NonEmptyString>,
        accepted_actions: Vec<OperatorAction>,
    ) -> Self {
        Self {
            case_id,
            capability,
            category,
            prediction_outcome: TeacherCalibrationPredictionOutcome::new(false, false, false),
            shape_status: ShapeStatus::Failed,
            expected_action_rationale,
            failure_message,
            predicted_action: None,
            accepted_actions,
        }
    }

    pub fn case_id(&self) -> &CalibrationCaseId {
        &self.case_id
    }

    pub fn capability(&self) -> CalibrationCapability {
        self.capability
    }

    pub fn category(&self) -> CalibrationCaseCategory {
        self.category
    }

    pub fn matched(&self) -> bool {
        self.prediction_outcome.matched()
    }

    pub fn tool_matched(&self) -> bool {
        self.prediction_outcome.tool_matched()
    }

    pub fn contract_valid(&self) -> bool {
        self.prediction_outcome.contract_valid()
    }

    pub fn shape_failed(&self) -> bool {
        self.shape_status.is_failed()
    }

    pub fn expected_action_rationale(&self) -> Option<&ExpectedActionRationale> {
        self.expected_action_rationale.as_ref()
    }

    pub fn failure_message(&self) -> Option<&NonEmptyString> {
        self.failure_message.as_ref()
    }

    pub fn predicted_action(&self) -> Option<&OperatorAction> {
        self.predicted_action.as_ref()
    }

    pub fn accepted_actions(&self) -> &[OperatorAction] {
        &self.accepted_actions
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShapeStatus {
    Valid,
    Failed,
}

impl ShapeStatus {
    fn is_failed(self) -> bool {
        self == Self::Failed
    }
}
