//! External audit signals attached to a corpus-quality evaluation.

use operator_shared_domain::value_objects::example_count::ExampleCount;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CorpusAuditSnapshot {
    schema_parse_failures: ExampleCount,
    action_parse_failures: ExampleCount,
    mode_safety_failures: ExampleCount,
    reference_safety_failures: ExampleCount,
    scope_safety_failures: ExampleCount,
    pagination_safety_failures: ExampleCount,
    write_proof_failures: ExampleCount,
    gold_leak_findings: ExampleCount,
    replay_failures: ExampleCount,
    replay_smoke_recorded: bool,
    frontier_ceiling_recorded: bool,
}

impl CorpusAuditSnapshot {
    pub fn clean() -> Self {
        Self {
            replay_smoke_recorded: true,
            frontier_ceiling_recorded: true,
            ..Self::default()
        }
    }

    #[must_use]
    pub fn with_schema_parse_failures(mut self, count: ExampleCount) -> Self {
        self.schema_parse_failures = count;
        self
    }

    #[must_use]
    pub fn with_action_parse_failures(mut self, count: ExampleCount) -> Self {
        self.action_parse_failures = count;
        self
    }

    #[must_use]
    pub fn with_mode_safety_failures(mut self, count: ExampleCount) -> Self {
        self.mode_safety_failures = count;
        self
    }

    #[must_use]
    pub fn with_reference_safety_failures(mut self, count: ExampleCount) -> Self {
        self.reference_safety_failures = count;
        self
    }

    #[must_use]
    pub fn with_scope_safety_failures(mut self, count: ExampleCount) -> Self {
        self.scope_safety_failures = count;
        self
    }

    #[must_use]
    pub fn with_pagination_safety_failures(mut self, count: ExampleCount) -> Self {
        self.pagination_safety_failures = count;
        self
    }

    #[must_use]
    pub fn with_write_proof_failures(mut self, count: ExampleCount) -> Self {
        self.write_proof_failures = count;
        self
    }

    #[must_use]
    pub fn with_gold_leak_findings(mut self, count: ExampleCount) -> Self {
        self.gold_leak_findings = count;
        self
    }

    #[must_use]
    pub fn with_replay_failures(mut self, count: ExampleCount) -> Self {
        self.replay_failures = count;
        self
    }

    #[must_use]
    pub fn without_replay_smoke(mut self) -> Self {
        self.replay_smoke_recorded = false;
        self
    }

    #[must_use]
    pub fn without_frontier_ceiling(mut self) -> Self {
        self.frontier_ceiling_recorded = false;
        self
    }

    pub fn schema_parse_failures(&self) -> ExampleCount {
        self.schema_parse_failures
    }

    pub fn action_parse_failures(&self) -> ExampleCount {
        self.action_parse_failures
    }

    pub fn mode_safety_failures(&self) -> ExampleCount {
        self.mode_safety_failures
    }

    pub fn reference_safety_failures(&self) -> ExampleCount {
        self.reference_safety_failures
    }

    pub fn scope_safety_failures(&self) -> ExampleCount {
        self.scope_safety_failures
    }

    pub fn pagination_safety_failures(&self) -> ExampleCount {
        self.pagination_safety_failures
    }

    pub fn write_proof_failures(&self) -> ExampleCount {
        self.write_proof_failures
    }

    pub fn gold_leak_findings(&self) -> ExampleCount {
        self.gold_leak_findings
    }

    pub fn replay_failures(&self) -> ExampleCount {
        self.replay_failures
    }

    pub fn replay_smoke_recorded(&self) -> bool {
        self.replay_smoke_recorded
    }

    pub fn frontier_ceiling_recorded(&self) -> bool {
        self.frontier_ceiling_recorded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clean_records_required_external_gates() {
        let audit = CorpusAuditSnapshot::clean();
        assert!(audit.replay_smoke_recorded());
        assert!(audit.frontier_ceiling_recorded());
        assert_eq!(audit.schema_parse_failures().as_usize(), 0);
    }

    #[test]
    fn builder_methods_set_failure_counts() {
        let audit = CorpusAuditSnapshot::clean()
            .with_schema_parse_failures(ExampleCount::new(1))
            .without_frontier_ceiling();
        assert_eq!(audit.schema_parse_failures().as_usize(), 1);
        assert!(!audit.frontier_ceiling_recorded());
        assert_eq!(audit.clone(), audit);
    }
}
