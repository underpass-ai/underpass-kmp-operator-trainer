//! Per-case generation metric. Captures the case identifier, the expected
//! minimum and how many trajectories were actually generated, plus a
//! derived flag for downstream coverage checks.

use operator_shared_domain::ids::synthetic_case_id::SyntheticCaseId;
use operator_shared_domain::value_objects::example_count::ExampleCount;
use operator_shared_domain::value_objects::positive_count::PositiveCount;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyntheticCaseGenerationMetric {
    case_id: SyntheticCaseId,
    expected_minimum: PositiveCount,
    generated: ExampleCount,
}

impl SyntheticCaseGenerationMetric {
    pub fn new(
        case_id: SyntheticCaseId,
        expected_minimum: PositiveCount,
        generated: ExampleCount,
    ) -> Self {
        Self {
            case_id,
            expected_minimum,
            generated,
        }
    }

    pub fn case_id(&self) -> &SyntheticCaseId {
        &self.case_id
    }

    pub fn expected_minimum(&self) -> PositiveCount {
        self.expected_minimum
    }

    pub fn generated(&self) -> ExampleCount {
        self.generated
    }

    pub fn satisfies_minimum(&self) -> bool {
        self.generated.as_usize() >= self.expected_minimum.as_usize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metric(expected: usize, generated: usize) -> SyntheticCaseGenerationMetric {
        SyntheticCaseGenerationMetric::new(
            SyntheticCaseId::parse("case:1").unwrap(),
            PositiveCount::parse(expected, "minimum").unwrap(),
            ExampleCount::new(generated),
        )
    }

    #[test]
    fn satisfies_minimum_when_generated_meets_or_exceeds() {
        assert!(metric(3, 3).satisfies_minimum());
        assert!(metric(3, 5).satisfies_minimum());
    }

    #[test]
    fn falls_below_minimum_when_generated_is_smaller() {
        assert!(!metric(5, 2).satisfies_minimum());
    }
}
