//! Composite validator that evaluates corpus-quality specifications in order.

use operator_shared_domain::specifications::specification::Specification;

use crate::quality::corpus_quality_validator::CorpusQualityValidator;
use crate::quality::corpus_quality_violations::CorpusQualityViolations;
use crate::quality::corpus_snapshot::CorpusSnapshot;
use crate::specifications::action_parse_spec::ActionParseSpec;
use crate::specifications::contract_coverage_spec::ContractCoverageSpec;
use crate::specifications::duplicate_audit_spec::DuplicateAuditSpec;
use crate::specifications::episode_split_spec::EpisodeSplitSpec;
use crate::specifications::frontier_ceiling_spec::FrontierCeilingSpec;
use crate::specifications::mode_safety_spec::ModeSafetySpec;
use crate::specifications::no_gold_audit_spec::NoGoldAuditSpec;
use crate::specifications::pagination_safety_spec::PaginationSafetySpec;
use crate::specifications::reference_safety_spec::ReferenceSafetySpec;
use crate::specifications::replay_smoke_spec::ReplaySmokeSpec;
use crate::specifications::schema_parse_spec::SchemaParseSpec;
use crate::specifications::scope_safety_spec::ScopeSafetySpec;
use crate::specifications::write_proof_spec::WriteProofSpec;

pub struct CompositeCorpusQualityValidator {
    specs: Vec<Box<dyn Specification<CorpusSnapshot>>>,
}

impl std::fmt::Debug for CompositeCorpusQualityValidator {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompositeCorpusQualityValidator")
            .field("specs_len", &self.specs.len())
            .finish()
    }
}

impl CompositeCorpusQualityValidator {
    pub fn new(specs: Vec<Box<dyn Specification<CorpusSnapshot>>>) -> Self {
        Self { specs }
    }

    pub fn default_strict() -> Self {
        Self::new(vec![
            Box::new(SchemaParseSpec::new()),
            Box::new(ActionParseSpec::new()),
            Box::new(ContractCoverageSpec::new()),
            Box::new(ModeSafetySpec::new()),
            Box::new(ReferenceSafetySpec::new()),
            Box::new(ScopeSafetySpec::new()),
            Box::new(PaginationSafetySpec::new()),
            Box::new(WriteProofSpec::new()),
            Box::new(NoGoldAuditSpec::new()),
            Box::new(EpisodeSplitSpec::new()),
            Box::new(DuplicateAuditSpec::new()),
            Box::new(ReplaySmokeSpec::new()),
            Box::new(FrontierCeilingSpec::new()),
        ])
    }

    pub fn spec_count(&self) -> usize {
        self.specs.len()
    }
}

impl CorpusQualityValidator for CompositeCorpusQualityValidator {
    fn validate(&self, snapshot: &CorpusSnapshot) -> Result<(), CorpusQualityViolations> {
        let mut violations = CorpusQualityViolations::new();
        for spec in &self.specs {
            if let Err(violation) = spec.evaluate(snapshot) {
                violations.push(violation);
            }
        }
        if violations.is_empty() {
            Ok(())
        } else {
            Err(violations)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quality::test_support::{full_quality_snapshot, inspect_snapshot_without_split};

    #[test]
    fn default_strict_registers_all_specs() {
        let validator = CompositeCorpusQualityValidator::default_strict();
        assert_eq!(validator.spec_count(), 13);
        assert!(format!("{validator:?}").contains("CompositeCorpusQualityValidator"));
    }

    #[test]
    fn validates_clean_snapshot() {
        let validator = CompositeCorpusQualityValidator::default_strict();
        validator
            .validate(&full_quality_snapshot())
            .expect("full snapshot passes");
    }

    #[test]
    fn accumulates_multiple_violations() {
        let validator = CompositeCorpusQualityValidator::default_strict();
        let violations = validator
            .validate(&inspect_snapshot_without_split())
            .expect_err("partial snapshot violates several specs");
        assert!(violations.len() >= 2);
    }
}
