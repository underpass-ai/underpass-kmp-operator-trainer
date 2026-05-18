//! End-to-end test: build a blueprint covering every `KmpMcpCapability`,
//! wire the application use case to the in-memory generator and assert
//! that the resulting report covers every capability and satisfies its
//! minimums.

use operator_shared_domain::contract::action_contract_validator::ActionContractValidator;
use operator_shared_domain::contract::composite_action_contract_validator::CompositeActionContractValidator;
use operator_shared_domain::ids::dataset_id::DatasetId;
use operator_shared_domain::value_objects::positive_count::PositiveCount;
use operator_synthetic_application::use_cases::generate_synthetic_dataset_use_case::GenerateSyntheticDatasetUseCase;
use operator_synthetic_domain::capability::kmp_mcp_capability::KmpMcpCapability;
use operator_synthetic_domain::dataset::synthetic_dataset_blueprint::SyntheticDatasetBlueprint;
use operator_synthetic_infra::generators::in_memory_synthetic_case_generator::InMemorySyntheticCaseGenerator;

#[test]
fn generates_a_full_coverage_dataset_with_in_memory_adapter() {
    let blueprint = SyntheticDatasetBlueprint::for_all_capabilities(
        DatasetId::parse("dataset:e2e").unwrap(),
        PositiveCount::parse(2, "minimum").unwrap(),
    )
    .unwrap();
    let use_case = GenerateSyntheticDatasetUseCase::new(InMemorySyntheticCaseGenerator::new());

    let report = use_case.execute(&blueprint).unwrap();

    let expected_trajectories = KmpMcpCapability::ALL.len() * 2;
    assert_eq!(report.dataset().len(), expected_trajectories);
    assert_eq!(report.total_generated().as_usize(), expected_trajectories);
    assert_eq!(report.case_metrics().len(), KmpMcpCapability::ALL.len());
    assert!(report.every_case_satisfies_minimum());
}

#[test]
fn every_trajectory_in_the_full_dataset_passes_the_contract_validator() {
    let blueprint = SyntheticDatasetBlueprint::for_all_capabilities(
        DatasetId::parse("dataset:contract").unwrap(),
        PositiveCount::parse(1, "minimum").unwrap(),
    )
    .unwrap();
    let use_case = GenerateSyntheticDatasetUseCase::new(InMemorySyntheticCaseGenerator::new());
    let validator = CompositeActionContractValidator::default_strict();

    let report = use_case.execute(&blueprint).unwrap();
    for trajectory in report.dataset().trajectories() {
        validator
            .validate(
                trajectory.target_action(),
                trajectory.mode(),
                trajectory.visible_state(),
            )
            .unwrap_or_else(|violations| {
                panic!(
                    "trajectory {} violated contract: {:?}",
                    trajectory.id().as_str(),
                    violations.as_slice(),
                )
            });
    }
}
