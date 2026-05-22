//! Mapper from realistic corpus application reports to JSON DTOs.

use std::collections::BTreeMap;

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

    pub fn drop_to_dto(entry: &DropEntry) -> DropEntryDto {
        DropEntryDto {
            scenario_id: entry.scenario_id().as_str().to_string(),
            target: entry.target().name().to_string(),
            reason: entry.reason().kind().as_str().to_string(),
            message: entry.reason().message(),
        }
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
