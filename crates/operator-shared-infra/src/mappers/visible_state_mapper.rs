use std::collections::BTreeSet;

use operator_shared_contract::visible_state_dto::{BudgetSnapshotDto, VisibleStateDto};
use operator_shared_domain::cursor::cursor::Cursor;
use operator_shared_domain::value_objects::dimension_ref::DimensionRef;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::visible_state::budget_snapshot::{BudgetField, BudgetSnapshot};
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
        Ok(VisibleState::assemble(
            known_refs,
            known_dimensions,
            active_cursor,
            budget_to_domain(dto.budget),
        ))
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
        }
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
