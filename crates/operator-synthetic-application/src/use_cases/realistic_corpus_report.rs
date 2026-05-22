//! Outcome report for one lenient realistic corpus production run.

use std::collections::BTreeMap;

use operator_shared_domain::trajectory::training_trajectory::TrainingTrajectory;
use operator_synthetic_domain::case::synthetic_generation_target::SyntheticGenerationTarget;

use crate::use_cases::drop_entry::DropEntry;
use crate::use_cases::drop_reason_kind::DropReasonKind;
use crate::use_cases::max_drop_rate::MaxDropRate;

#[derive(Debug, Clone, PartialEq)]
pub struct RealisticCorpusReport {
    accepted: Vec<TrainingTrajectory>,
    dropped: Vec<DropEntry>,
    max_drop_rate: MaxDropRate,
}

impl RealisticCorpusReport {
    pub fn from_outcome(
        accepted: Vec<TrainingTrajectory>,
        dropped: Vec<DropEntry>,
        max_drop_rate: MaxDropRate,
    ) -> Self {
        Self {
            accepted,
            dropped,
            max_drop_rate,
        }
    }

    pub fn accepted(&self) -> &[TrainingTrajectory] {
        &self.accepted
    }

    pub fn dropped(&self) -> &[DropEntry] {
        &self.dropped
    }

    pub fn total_scenarios(&self) -> usize {
        self.accepted.len() + self.dropped.len()
    }

    pub fn accepted_count(&self) -> usize {
        self.accepted.len()
    }

    pub fn dropped_count(&self) -> usize {
        self.dropped.len()
    }

    pub fn drop_rate(&self) -> f64 {
        let total = self.total_scenarios();
        if total == 0 {
            return 1.0;
        }
        let dropped =
            u32::try_from(self.dropped.len()).expect("realistic corpus dropped row count fits u32");
        let total = u32::try_from(total).expect("realistic corpus row count fits u32");
        f64::from(dropped) / f64::from(total)
    }

    pub fn max_drop_rate(&self) -> MaxDropRate {
        self.max_drop_rate
    }

    pub fn gate_passed(&self) -> bool {
        self.total_scenarios() > 0 && self.drop_rate() <= self.max_drop_rate.as_f64()
    }

    pub fn gate_failure_reason(&self) -> Option<String> {
        if self.total_scenarios() == 0 {
            return Some("no scenarios were processed".to_string());
        }
        let drop_rate = self.drop_rate();
        let max = self.max_drop_rate.as_f64();
        (drop_rate > max).then(|| format!("drop_rate {drop_rate:.4} > max_drop_rate {max:.4}"))
    }

    pub fn dropped_by_reason(&self) -> BTreeMap<DropReasonKind, usize> {
        let mut out = BTreeMap::new();
        for entry in &self.dropped {
            *out.entry(entry.reason().kind()).or_insert(0) += 1;
        }
        out
    }

    pub fn per_target_accepted(&self) -> BTreeMap<SyntheticGenerationTarget, usize> {
        let mut out = BTreeMap::new();
        for row in &self.accepted {
            *out.entry(SyntheticGenerationTarget::from_action(row.target_action()))
                .or_insert(0) += 1;
        }
        out
    }

    pub fn per_target_total(&self) -> BTreeMap<SyntheticGenerationTarget, usize> {
        let mut out = self.per_target_accepted();
        for entry in &self.dropped {
            *out.entry(entry.target()).or_insert(0) += 1;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::value_objects::subject_hash::SubjectHash;
    use operator_synthetic_domain::capability::kmp_mcp_capability::KmpMcpCapability;

    use crate::ports::scenario_id::ScenarioId;
    use crate::use_cases::drop_reason::DropReason;

    #[test]
    fn empty_report_fails_gate() {
        let report =
            RealisticCorpusReport::from_outcome(vec![], vec![], MaxDropRate::parse(0.05).unwrap());
        assert!(!report.gate_passed());
        assert!((report.drop_rate() - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn drop_within_gate_passes() {
        let report = RealisticCorpusReport::from_outcome(
            vec![],
            vec![DropEntry::new(
                ScenarioId::parse("scenario:1").unwrap(),
                SyntheticGenerationTarget::from(KmpMcpCapability::Inspect),
                DropReason::TeacherError {
                    message: "timeout".to_string(),
                },
                None,
                subject_hash(),
                None,
            )],
            MaxDropRate::parse(1.0).unwrap(),
        );
        assert!(report.gate_passed());
        assert_eq!(
            report
                .dropped_by_reason()
                .get(&DropReasonKind::TeacherError),
            Some(&1)
        );
    }

    fn subject_hash() -> SubjectHash {
        SubjectHash::parse("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
            .unwrap()
    }
}
