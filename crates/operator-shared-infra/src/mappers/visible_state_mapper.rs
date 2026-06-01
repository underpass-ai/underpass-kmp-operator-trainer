use std::collections::BTreeSet;

use operator_shared_contract::budget_snapshot_dto::BudgetSnapshotDto;
use operator_shared_contract::coverage_deviation_snapshot_dto::CoverageDeviationSnapshotDto;
use operator_shared_contract::visible_state_dto::VisibleStateDto;
use operator_shared_domain::cursor::cursor::Cursor;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::value_objects::dimension_ref::DimensionRef;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::visible_state::budget_field::BudgetField;
use operator_shared_domain::visible_state::budget_snapshot::BudgetSnapshot;
use operator_shared_domain::visible_state::coverage_deviation_snapshot::CoverageDeviationSnapshot;
use operator_shared_domain::visible_state::visible_state::VisibleState;

use crate::mappers::cursor_mapper::CursorMapper;
use crate::mappers::mapping_error::MappingError;

#[derive(Debug)]
pub struct VisibleStateMapper;

impl VisibleStateMapper {
    pub fn to_domain(dto: &VisibleStateDto) -> Result<VisibleState, MappingError> {
        let mut known_refs: BTreeSet<MemoryRef> = BTreeSet::new();
        for raw in &dto.known_refs {
            known_refs.insert(MemoryRef::parse(raw.clone())?);
        }
        let mut known_dimensions: BTreeSet<DimensionRef> = BTreeSet::new();
        for raw in &dto.known_dimensions {
            known_dimensions.insert(DimensionRef::parse(raw.clone())?);
        }
        let active_cursor: Option<Cursor> = match dto.active_cursor.as_ref() {
            Some(cursor_dto) => Some(CursorMapper::to_domain(cursor_dto)?),
            None => None,
        };
        let mut candidate_abouts = Vec::with_capacity(dto.candidate_abouts.len());
        for raw in &dto.candidate_abouts {
            candidate_abouts.push(AboutId::parse(raw.clone())?);
        }
        Ok(VisibleState::assemble(
            known_refs,
            known_dimensions,
            active_cursor,
            budget_to_domain(dto.budget),
        )
        .with_coverage_deviation(coverage_deviation_to_domain(dto.coverage_deviation))
        .with_candidate_abouts(candidate_abouts))
    }

    pub fn to_dto(domain: &VisibleState) -> VisibleStateDto {
        VisibleStateDto {
            known_refs: domain
                .known_refs()
                .iter()
                .map(|r| r.as_str().to_string())
                .collect(),
            known_dimensions: domain
                .known_dimensions()
                .iter()
                .map(|r| r.as_str().to_string())
                .collect(),
            active_cursor: domain.active_cursor().map(CursorMapper::to_dto),
            budget: budget_to_dto(domain.budget()),
            coverage_deviation: coverage_deviation_to_dto(domain.coverage_deviation()),
            candidate_abouts: domain
                .candidate_abouts()
                .iter()
                .map(|a| a.as_str().to_string())
                .collect(),
        }
    }
}

fn coverage_deviation_to_domain(
    dto: Option<CoverageDeviationSnapshotDto>,
) -> CoverageDeviationSnapshot {
    match dto {
        Some(dto) => CoverageDeviationSnapshot::new(
            dto.deviation_milli,
            dto.saturated,
            dto.conflict_blocking,
        ),
        None => CoverageDeviationSnapshot::unknown(),
    }
}

fn coverage_deviation_to_dto(
    domain: CoverageDeviationSnapshot,
) -> Option<CoverageDeviationSnapshotDto> {
    if domain == CoverageDeviationSnapshot::unknown() {
        None
    } else {
        Some(CoverageDeviationSnapshotDto {
            deviation_milli: domain.deviation_milli(),
            saturated: domain.saturated(),
            conflict_blocking: domain.conflict_blocking(),
        })
    }
}

fn budget_to_domain(dto: BudgetSnapshotDto) -> BudgetSnapshot {
    let mut snapshot = BudgetSnapshot::unbounded();
    if let Some(value) = dto.calls_remaining {
        snapshot = snapshot.with_calls_remaining(value);
    }
    if let Some(value) = dto.tokens_remaining {
        snapshot = snapshot.with_tokens_remaining(value);
    }
    snapshot
}

fn budget_to_dto(domain: BudgetSnapshot) -> BudgetSnapshotDto {
    BudgetSnapshotDto {
        calls_remaining: match domain.calls_remaining() {
            BudgetField::Unbounded => None,
            BudgetField::Bounded(value) => Some(value),
        },
        tokens_remaining: match domain.tokens_remaining() {
            BudgetField::Unbounded => None,
            BudgetField::Bounded(value) => Some(value),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn candidate_abouts_round_trip_through_dto() {
        let state = VisibleState::assemble([], [], None, BudgetSnapshot::bounded(5, 4096))
            .with_candidate_abouts(vec![
                AboutId::parse("ctrl-v01").unwrap(),
                AboutId::parse("ctrl-v05").unwrap(),
            ]);
        let dto = VisibleStateMapper::to_dto(&state);
        assert_eq!(dto.candidate_abouts, vec!["ctrl-v01", "ctrl-v05"]);

        let back = VisibleStateMapper::to_domain(&dto).expect("round-trips");
        assert_eq!(back.candidate_abouts().len(), 2);
        assert_eq!(back.candidate_abouts()[0].as_str(), "ctrl-v01");
        assert_eq!(back.candidate_abouts()[1].as_str(), "ctrl-v05");
    }

    #[test]
    fn empty_candidate_abouts_is_omitted_from_the_wire() {
        let state = VisibleState::assemble([], [], None, BudgetSnapshot::bounded(1, 4096));
        let dto = VisibleStateMapper::to_dto(&state);
        assert!(dto.candidate_abouts.is_empty());
        // skip-if-empty keeps single-about wire snapshots unchanged.
        let json = serde_json::to_string(&dto).expect("serializes");
        assert!(!json.contains("candidate_abouts"), "must not appear: {json}");
    }
}
