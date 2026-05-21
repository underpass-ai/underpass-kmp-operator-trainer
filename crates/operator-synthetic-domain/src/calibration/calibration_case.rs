//! Gold-standard teacher calibration case.

use crate::calibration::accepted_actions::AcceptedActions;
use crate::calibration::calibration_capability::CalibrationCapability;
use crate::calibration::calibration_case_category::CalibrationCaseCategory;
use crate::calibration::calibration_case_id::CalibrationCaseId;
use crate::calibration::calibration_domain_theme::CalibrationDomainTheme;
use crate::calibration::calibration_subject::CalibrationSubject;
use crate::calibration::expected_action_rationale::ExpectedActionRationale;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalibrationCase {
    case_id: CalibrationCaseId,
    domain_theme: CalibrationDomainTheme,
    category: CalibrationCaseCategory,
    subject: CalibrationSubject,
    accepted_actions: AcceptedActions,
    expected_action_rationale: ExpectedActionRationale,
}

impl CalibrationCase {
    pub fn new(
        case_id: CalibrationCaseId,
        domain_theme: CalibrationDomainTheme,
        category: CalibrationCaseCategory,
        subject: CalibrationSubject,
        accepted_actions: AcceptedActions,
        expected_action_rationale: ExpectedActionRationale,
    ) -> Self {
        Self {
            case_id,
            domain_theme,
            category,
            subject,
            accepted_actions,
            expected_action_rationale,
        }
    }

    pub fn case_id(&self) -> &CalibrationCaseId {
        &self.case_id
    }

    pub fn domain_theme(&self) -> CalibrationDomainTheme {
        self.domain_theme
    }

    pub fn category(&self) -> CalibrationCaseCategory {
        self.category
    }

    pub fn subject(&self) -> &CalibrationSubject {
        &self.subject
    }

    pub fn accepted_actions(&self) -> &AcceptedActions {
        &self.accepted_actions
    }

    pub fn expected_action_rationale(&self) -> &ExpectedActionRationale {
        &self.expected_action_rationale
    }

    pub fn capability(&self) -> CalibrationCapability {
        self.accepted_actions.capability()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::action::operator_action::OperatorAction;
    use operator_shared_domain::action::stop_action::StopAction;
    use operator_shared_domain::action::stop_reason::StopReason;
    use operator_shared_domain::ids::about_id::AboutId;
    use operator_shared_domain::mode::allowed_tools::AllowedTools;
    use operator_shared_domain::mode::operator_mode::OperatorMode;
    use operator_shared_domain::value_objects::task_family::TaskFamily;
    use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
    use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
    use operator_shared_domain::visible_state::visible_state::VisibleState;

    #[test]
    fn exposes_case_parts() {
        let case = CalibrationCase::new(
            CalibrationCaseId::parse("calib:stop").unwrap(),
            CalibrationDomainTheme::TechnicalIncident,
            CalibrationCaseCategory::Happy,
            CalibrationSubject::new(
                AboutId::parse("about:stop").unwrap(),
                OperatorMode::Read,
                TaskFamily::parse("read.stop").unwrap(),
                TrajectoryGoal::parse("Stop with answer.").unwrap(),
                AllowedTools::for_mode(OperatorMode::Read),
                VisibleState::assemble([], [], None, BudgetSnapshot::unbounded()),
            )
            .unwrap(),
            AcceptedActions::new(vec![OperatorAction::Stop(
                StopAction::new(StopReason::AnswerReady, None, vec![]).unwrap(),
            )])
            .unwrap(),
            ExpectedActionRationale::parse("The answer is already proven.").unwrap(),
        );
        assert_eq!(case.capability(), CalibrationCapability::Stop);
        assert_eq!(case.case_id().as_str(), "calib:stop");
    }
}
