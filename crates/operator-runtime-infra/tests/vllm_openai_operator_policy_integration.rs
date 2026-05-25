use operator_runtime_application::ports::operator_policy_port::OperatorPolicy;
use operator_runtime_infra::adapters::vllm_openai_operator_policy::VllmOpenAiOperatorPolicy;
use operator_runtime_infra::adapters::vllm_operator_config::VllmOperatorConfig;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::mode::allowed_tools::AllowedTools;
use operator_shared_domain::mode::operator_mode::OperatorMode;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::task_family::TaskFamily;
use operator_shared_domain::value_objects::trajectory_goal::TrajectoryGoal;
use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
use operator_shared_domain::visible_state::visible_state::VisibleState;
use operator_synthetic_domain::calibration::calibration_subject::CalibrationSubject;

#[test]
#[ignore = "requires live operator endpoint; set OPERATOR_TEST_ENDPOINT, OPERATOR_TEST_CERT, and OPERATOR_TEST_KEY"]
fn predicts_real_action_from_live_endpoint() {
    let Ok(endpoint) = std::env::var("OPERATOR_TEST_ENDPOINT") else {
        eprintln!("OPERATOR_TEST_ENDPOINT not set; skipped");
        return;
    };
    let Ok(cert) = std::env::var("OPERATOR_TEST_CERT") else {
        eprintln!("OPERATOR_TEST_CERT not set; skipped");
        return;
    };
    let Ok(key) = std::env::var("OPERATOR_TEST_KEY") else {
        eprintln!("OPERATOR_TEST_KEY not set; skipped");
        return;
    };
    let config =
        VllmOperatorConfig::new(endpoint, "operator-v8.1.2").with_client_identity(cert, key);
    let policy = VllmOpenAiOperatorPolicy::new(&config).expect("policy builds");
    let target = MemoryRef::parse("node:1").unwrap();
    let subject = CalibrationSubject::new(
        AboutId::parse("about:test").unwrap(),
        OperatorMode::Read,
        TaskFamily::parse("runtime.single_step").unwrap(),
        TrajectoryGoal::parse("Inspect node one.").unwrap(),
        AllowedTools::for_mode(OperatorMode::Read),
        VisibleState::assemble([target], [], None, BudgetSnapshot::bounded(1, 4096)),
        None,
    )
    .unwrap();

    let action = policy.predict(&subject).expect("live prediction succeeds");

    assert!(action.tool().is_some());
}
