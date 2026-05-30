//! The teacher-backed operator policy forwards a synthetic teacher's decision
//! as the runtime loop's prediction and translates the error taxonomy.

use operator_runtime_application::errors::operator_policy_error::OperatorPolicyError;
use operator_runtime_application::ports::operator_policy_port::OperatorPolicy;
use operator_runtime_infra::adapters::teacher_backed_operator_policy::TeacherBackedOperatorPolicy;
use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::action::stop_action::StopAction;
use operator_shared_domain::action::stop_reason::StopReason;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::mode::allowed_tools::AllowedTools;
use operator_shared_domain::mode::operator_mode::OperatorMode;
use operator_shared_domain::value_objects::finish_reason::FinishReason;
use operator_shared_domain::value_objects::subject_hash::SubjectHash;
use operator_shared_domain::value_objects::task_family::TaskFamily;
use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
use operator_shared_domain::visible_state::visible_state::VisibleState;
use operator_synthetic_application::error::teacher_policy_error::TeacherPolicyError;
use operator_synthetic_application::ports::teacher_policy::TeacherPolicy;
use operator_synthetic_domain::calibration::calibration_subject::CalibrationSubject;
use operator_synthetic_domain::calibration::teacher_decision::TeacherDecision;

#[derive(Debug)]
struct StubTeacherPolicy {
    result: Result<TeacherDecision, TeacherPolicyError>,
}

impl TeacherPolicy for StubTeacherPolicy {
    fn decide(&self, _subject: &CalibrationSubject) -> Result<TeacherDecision, TeacherPolicyError> {
        self.result.clone()
    }
}

fn subject() -> CalibrationSubject {
    CalibrationSubject::new(
        AboutId::parse("about:test").unwrap(),
        OperatorMode::Read,
        TaskFamily::parse("runtime.window_expansion").unwrap(),
        TrajectoryGoal::parse("Widen the window until covered.").unwrap(),
        AllowedTools::for_mode(OperatorMode::Read),
        VisibleState::assemble([], [], None, BudgetSnapshot::bounded(3, 4096)),
        None,
    )
    .unwrap()
}

fn stop_decision() -> TeacherDecision {
    TeacherDecision::new(
        OperatorAction::Stop(StopAction::new(StopReason::AnswerReady, None, vec![]).unwrap()),
        FinishReason::Stop,
        SubjectHash::parse("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
            .unwrap(),
    )
}

#[test]
fn forwards_the_teacher_action_as_the_prediction() {
    let policy = TeacherBackedOperatorPolicy::new(StubTeacherPolicy {
        result: Ok(stop_decision()),
    });

    let action = policy.predict(&subject()).expect("prediction succeeds");

    assert!(matches!(action, OperatorAction::Stop(_)));
}

#[test]
fn maps_shape_errors_preserving_adapter_and_message() {
    let policy = TeacherBackedOperatorPolicy::new(StubTeacherPolicy {
        result: Err(TeacherPolicyError::Shape {
            adapter: "openai_compatible",
            message: "missing tool field".to_string(),
            finish_reason: None,
        }),
    });

    let error = policy
        .predict(&subject())
        .expect_err("shape error propagates");

    match error {
        OperatorPolicyError::Shape { message } => {
            assert!(message.contains("openai_compatible"));
            assert!(message.contains("missing tool field"));
        }
        other => panic!("expected a shape error, got {other:?}"),
    }
}

#[test]
fn maps_transport_errors_to_transport() {
    let policy = TeacherBackedOperatorPolicy::new(StubTeacherPolicy {
        result: Err(TeacherPolicyError::Transport {
            adapter: "openai_compatible",
            message: "connection refused".to_string(),
        }),
    });

    let error = policy
        .predict(&subject())
        .expect_err("transport error propagates");

    assert!(matches!(error, OperatorPolicyError::Transport { .. }));
}

#[test]
fn maps_api_errors_to_transport_with_code() {
    let policy = TeacherBackedOperatorPolicy::new(StubTeacherPolicy {
        result: Err(TeacherPolicyError::ApiError {
            adapter: "openai_compatible",
            code: Some("429".to_string()),
            message: "rate limited".to_string(),
        }),
    });

    let error = policy
        .predict(&subject())
        .expect_err("api error propagates");

    match error {
        OperatorPolicyError::Transport { message } => {
            assert!(message.contains("429"));
            assert!(message.contains("rate limited"));
        }
        other => panic!("expected a transport error, got {other:?}"),
    }
}

#[test]
fn maps_protocol_errors_to_protocol() {
    let policy = TeacherBackedOperatorPolicy::new(StubTeacherPolicy {
        result: Err(TeacherPolicyError::Protocol {
            adapter: "openai_compatible",
            message: "unparseable body".to_string(),
        }),
    });

    let error = policy
        .predict(&subject())
        .expect_err("protocol error propagates");

    assert!(matches!(error, OperatorPolicyError::Protocol { .. }));
}

#[test]
fn maps_truncated_response_to_protocol() {
    let policy = TeacherBackedOperatorPolicy::new(StubTeacherPolicy {
        result: Err(TeacherPolicyError::TruncatedResponse {
            adapter: "openai_compatible",
            finish_reason: FinishReason::parse("length"),
            content_len: 42,
            raw_content_tail: "...".to_string(),
            request_bytes: 100,
            response_bytes: 200,
            elapsed_ms: 12,
            request_id: None,
        }),
    });

    let error = policy
        .predict(&subject())
        .expect_err("truncated response propagates");

    assert!(matches!(error, OperatorPolicyError::Protocol { .. }));
}
