//! `GenerateSyntheticDatasetUseCase` walks every case in a blueprint,
//! delegates generation to a `SyntheticCaseGenerator` adapter and
//! assembles a `SyntheticDatasetGenerationReport`.
//!
//! Per-case minimum enforcement is **not** a hard error: the report
//! exposes `every_case_satisfies_minimum()` so the caller can decide
//! whether partial coverage is acceptable. Adapter failures and domain
//! constructor failures propagate immediately.

use operator_shared_domain::value_objects::example_count::ExampleCount;
use operator_synthetic_domain::case::synthetic_case_generation_metric::SyntheticCaseGenerationMetric;
use operator_synthetic_domain::dataset::synthetic_dataset::SyntheticDataset;
use operator_synthetic_domain::dataset::synthetic_dataset_blueprint::SyntheticDatasetBlueprint;
use operator_synthetic_domain::dataset::synthetic_dataset_generation_report::SyntheticDatasetGenerationReport;

use crate::error::generate_synthetic_dataset_error::GenerateSyntheticDatasetError;
use crate::ports::synthetic_case_generator::SyntheticCaseGenerator;

#[derive(Debug)]
pub struct GenerateSyntheticDatasetUseCase<G: SyntheticCaseGenerator> {
    generator: G,
}

impl<G: SyntheticCaseGenerator> GenerateSyntheticDatasetUseCase<G> {
    pub fn new(generator: G) -> Self {
        Self { generator }
    }

    pub fn execute(
        &self,
        blueprint: &SyntheticDatasetBlueprint,
    ) -> Result<SyntheticDatasetGenerationReport, GenerateSyntheticDatasetError> {
        let mut all_trajectories = Vec::new();
        let mut case_metrics = Vec::with_capacity(blueprint.cases().len());
        for spec in blueprint.cases() {
            let trajectories = self.generator.generate(spec)?;
            let generated = ExampleCount::new(trajectories.len());
            case_metrics.push(SyntheticCaseGenerationMetric::new(
                spec.case_id().clone(),
                spec.minimum_examples(),
                generated,
            ));
            all_trajectories.extend(trajectories);
        }
        let dataset = SyntheticDataset::new(blueprint.dataset_id().clone(), all_trajectories)?;
        Ok(SyntheticDatasetGenerationReport::new(dataset, case_metrics))
    }
}
