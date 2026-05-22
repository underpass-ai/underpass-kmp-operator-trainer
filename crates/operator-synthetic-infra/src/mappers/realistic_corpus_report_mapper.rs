//! Mapper from realistic corpus application reports to JSON DTOs.

use std::collections::BTreeMap;

use operator_shared_infra::mappers::mapping_error::MappingError;
use operator_shared_infra::mappers::operator_action_mapper::OperatorActionMapper;
use operator_synthetic_application::use_cases::drop_entry::DropEntry;
use operator_synthetic_application::use_cases::realistic_corpus_report::RealisticCorpusReport;

use crate::dto::drop_entry_dto::DropEntryDto;
use crate::dto::realistic_corpus_report_dto::RealisticCorpusReportDto;
use crate::dto::realistic_corpus_run_metadata_dto::RealisticCorpusRunMetadataDto;

#[derive(Debug)]
pub struct RealisticCorpusReportMapper;

impl RealisticCorpusReportMapper {
    pub fn to_dto(
        report: &RealisticCorpusReport,
        metadata: RealisticCorpusRunMetadataDto,
    ) -> RealisticCorpusReportDto {
        RealisticCorpusReportDto {
            predictor: metadata.predictor,
            run_id: metadata.run_id,
            scenarios_path: metadata.scenarios_path,
            scenarios_sha256: metadata.scenarios_sha256,
            prompt_path: metadata.prompt_path,
            prompt_sha256: metadata.prompt_sha256,
            api_base: metadata.api_base,
            model: metadata.model,
            temperature: metadata.temperature,
            started_at_unix: metadata.started_at_unix,
            finished_at_unix: metadata.finished_at_unix,
            total_scenarios: report.total_scenarios(),
            accepted_count: report.accepted_count(),
            dropped_count: report.dropped_count(),
            drop_rate: report.drop_rate(),
            max_drop_rate_gate: report.max_drop_rate().as_f64(),
            dropped_by_reason: report
                .dropped_by_reason()
                .into_iter()
                .map(|(kind, count)| (kind.as_str().to_string(), count))
                .collect(),
            per_target_accepted: target_map_to_string(report.per_target_accepted()),
            per_target_total: target_map_to_string(report.per_target_total()),
            gate_passed: report.gate_passed(),
            gate_failure_reason: report.gate_failure_reason(),
        }
    }

    pub fn drop_to_dto(entry: &DropEntry) -> Result<DropEntryDto, MappingError> {
        let predicted_action = entry
            .predicted_action()
            .map(OperatorActionMapper::to_dto)
            .transpose()?;
        Ok(DropEntryDto {
            scenario_id: entry.scenario_id().as_str().to_string(),
            target: entry.target().name().to_string(),
            reason: entry.reason().kind().as_str().to_string(),
            message: entry.reason().message(),
            predicted_action,
            subject_hash: entry.subject_hash().as_str().to_string(),
            teacher_finish_reason: entry
                .teacher_finish_reason()
                .map(|finish_reason| finish_reason.as_str().to_string()),
        })
    }
}

fn target_map_to_string(
    input: BTreeMap<
        operator_synthetic_domain::case::synthetic_generation_target::SyntheticGenerationTarget,
        usize,
    >,
) -> BTreeMap<String, usize> {
    input
        .into_iter()
        .map(|(target, count)| (target.name().to_string(), count))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::action::operator_action::OperatorAction;
    use operator_shared_domain::action::tool_call_action::ToolCallAction;
    use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
    use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
    use operator_shared_domain::value_objects::finish_reason::FinishReason;
    use operator_shared_domain::value_objects::memory_ref::MemoryRef;
    use operator_shared_domain::value_objects::subject_hash::SubjectHash;
    use operator_synthetic_application::ports::scenario_id::ScenarioId;
    use operator_synthetic_application::use_cases::drop_reason::DropReason;
    use operator_synthetic_domain::capability::kmp_mcp_capability::KmpMcpCapability;
    use operator_synthetic_domain::case::synthetic_generation_target::SyntheticGenerationTarget;

    #[test]
    fn drop_to_dto_persists_predicted_action_context() {
        let entry = DropEntry::new(
            ScenarioId::parse("scenario:inspect").expect("scenario id parses"),
            SyntheticGenerationTarget::from(KmpMcpCapability::Inspect),
            DropReason::TargetMismatch {
                expected: SyntheticGenerationTarget::from(KmpMcpCapability::Inspect),
                got_kind: "kernel_wake".to_string(),
            },
            Some(OperatorAction::ToolCall(ToolCallAction::new(
                ToolArguments::Inspect(InspectArguments::new(
                    MemoryRef::parse("node:target").expect("ref parses"),
                )),
            ))),
            subject_hash(),
            Some(FinishReason::Stop),
        );

        let dto = RealisticCorpusReportMapper::drop_to_dto(&entry).expect("drop maps");

        assert_eq!(dto.scenario_id, "scenario:inspect");
        assert_eq!(dto.reason, "target_mismatch");
        assert!(dto.predicted_action.is_some());
        assert_eq!(dto.subject_hash, subject_hash().as_str());
        assert_eq!(dto.teacher_finish_reason.as_deref(), Some("stop"));
    }

    fn subject_hash() -> SubjectHash {
        SubjectHash::parse("0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef")
            .expect("subject hash parses")
    }
}
