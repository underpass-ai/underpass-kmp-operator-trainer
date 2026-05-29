//! `OperatorPolicy` decorator that serves the anonymized operator on raw,
//! domain-bearing requests.
//!
//! The release model is trained on opaque refs (see `ref_anonymization`). This
//! decorator closes the train/serve gap (the V6 "raw-ref de-anonymization"
//! design): it anonymizes the model-facing subject before delegating to the
//! inner policy (e.g. the vLLM policy) and de-anonymizes the predicted action's
//! refs back to the real refs the caller and KMP understand.

use std::sync::Arc;

use operator_runtime_application::errors::operator_policy_error::OperatorPolicyError;
use operator_runtime_application::ports::operator_policy_port::OperatorPolicy;
use operator_shared_contract::operator_action_dto::OperatorActionDto;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_infra::mappers::operator_action_mapper::OperatorActionMapper;
use operator_synthetic_domain::calibration::calibration_subject::CalibrationSubject;
use operator_synthetic_infra::dto::calibration_subject_dto::CalibrationSubjectDto;
use operator_synthetic_infra::mappers::calibration_subject_mapper::CalibrationSubjectMapper;

use crate::adapters::ref_anonymization::RefAnonymization;

#[derive(Debug)]
pub struct AnonymizingOperatorPolicy {
    inner: Arc<dyn OperatorPolicy>,
}

impl AnonymizingOperatorPolicy {
    pub fn new(inner: Arc<dyn OperatorPolicy>) -> Self {
        Self { inner }
    }
}

impl OperatorPolicy for AnonymizingOperatorPolicy {
    fn predict(&self, subject: &CalibrationSubject) -> Result<OperatorAction, OperatorPolicyError> {
        // Anonymize the model-facing subject (real refs -> opaque ids).
        let subject_dto = CalibrationSubjectMapper::to_dto(subject);
        let subject_value =
            serde_json::to_value(&subject_dto).map_err(|err| OperatorPolicyError::Protocol {
                message: format!("anonymize: serialize subject: {err}"),
            })?;
        let known_refs: Vec<String> = subject
            .visible_state()
            .known_refs()
            .iter()
            .map(|r| r.as_str().to_string())
            .collect();
        let anonymization =
            RefAnonymization::build(subject.about().as_str(), &known_refs, &subject_value);
        let anon_value = anonymization.anonymize(&subject_value);
        let anon_dto: CalibrationSubjectDto =
            serde_json::from_value(anon_value).map_err(|err| OperatorPolicyError::Protocol {
                message: format!("anonymize: deserialize subject: {err}"),
            })?;
        let anon_subject = CalibrationSubjectMapper::to_domain(&anon_dto).map_err(|err| {
            OperatorPolicyError::Protocol {
                message: format!("anonymize: rebuild subject: {err}"),
            }
        })?;

        // Delegate to the wrapped policy on the anonymized subject.
        let action = self.inner.predict(&anon_subject)?;

        // De-anonymize the predicted action (opaque ids -> real refs).
        let action_dto =
            OperatorActionMapper::to_dto(&action).map_err(|err| OperatorPolicyError::Shape {
                message: format!("deanonymize: serialize action: {err}"),
            })?;
        let action_value =
            serde_json::to_value(&action_dto).map_err(|err| OperatorPolicyError::Shape {
                message: format!("deanonymize: action to value: {err}"),
            })?;
        let real_value = anonymization.deanonymize(&action_value);
        let real_dto: OperatorActionDto =
            serde_json::from_value(real_value).map_err(|err| OperatorPolicyError::Shape {
                message: format!("deanonymize: action from value: {err}"),
            })?;
        OperatorActionMapper::to_domain(&real_dto).map_err(|err| OperatorPolicyError::Shape {
            message: format!("deanonymize: rebuild action: {err}"),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::action::tool_call_action::ToolCallAction;
    use operator_shared_domain::ids::about_id::AboutId;
    use operator_shared_domain::mode::allowed_tools::AllowedTools;
    use operator_shared_domain::mode::operator_mode::OperatorMode;
    use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
    use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
    use operator_shared_domain::value_objects::memory_ref::MemoryRef;
    use operator_shared_domain::value_objects::task_family::TaskFamily;
    use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
    use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
    use operator_shared_domain::visible_state::visible_state::VisibleState;

    const SCOPE: &str = "about:incident:checkout-latency:case-1";
    const NODE: &str = "about:incident:checkout-latency:case-1:node:symptom:000";

    /// Inner policy that asserts it received an ANONYMIZED subject, then inspects
    /// the (opaque) first known ref — exercising both directions of the decorator.
    #[derive(Debug)]
    struct AssertAnonymizedThenInspect;

    impl OperatorPolicy for AssertAnonymizedThenInspect {
        fn predict(
            &self,
            subject: &CalibrationSubject,
        ) -> Result<OperatorAction, OperatorPolicyError> {
            assert_eq!(
                subject.about().as_str(),
                "about_0001",
                "inner must see an anonymized scope"
            );
            let first = subject
                .visible_state()
                .known_refs()
                .iter()
                .next()
                .expect("known ref present");
            assert_eq!(first.as_str(), "ref_0001", "inner must see an opaque ref");
            // the goal's embedded ref must also be anonymized
            assert!(
                !subject.goal().as_str().contains("about:"),
                "goal must not leak a domain ref"
            );
            Ok(OperatorAction::ToolCall(ToolCallAction::new(
                ToolArguments::Inspect(InspectArguments::new(first.clone())),
            )))
        }
    }

    fn subject() -> CalibrationSubject {
        CalibrationSubject::new(
            AboutId::parse(SCOPE).unwrap(),
            OperatorMode::Read,
            TaskFamily::parse("runtime.single_step").unwrap(),
            TrajectoryGoal::parse(format!("Inspect node {NODE} to read its metadata.")).unwrap(),
            AllowedTools::for_mode(OperatorMode::Read),
            VisibleState::assemble(
                [MemoryRef::parse(NODE).unwrap()],
                [],
                None,
                BudgetSnapshot::bounded(2, 1200),
            ),
            None,
        )
        .unwrap()
    }

    #[test]
    fn anonymizes_inbound_and_deanonymizes_the_predicted_action() {
        let policy = AnonymizingOperatorPolicy::new(Arc::new(AssertAnonymizedThenInspect));
        let action = policy.predict(&subject()).expect("predict succeeds");

        let OperatorAction::ToolCall(call) = action else {
            panic!("expected a tool call");
        };
        let ToolArguments::Inspect(args) = call.arguments() else {
            panic!("expected inspect");
        };
        // the inner inspected the opaque ref_0001; the decorator restored the real ref
        assert_eq!(args.target().as_str(), NODE);
    }
}
