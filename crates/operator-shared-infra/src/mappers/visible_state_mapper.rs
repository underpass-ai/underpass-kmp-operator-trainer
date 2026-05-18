use operator_shared_contract::visible_state_dto::{BudgetSnapshotDto, VisibleStateDto};
use operator_shared_domain::value_objects::dimension_ref::DimensionRef;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::visible_state::budget_snapshot::{BudgetField, BudgetSnapshot};
use operator_shared_domain::visible_state::visible_state::VisibleState;
use operator_shared_domain::visible_state::visible_state_builder::VisibleStateBuilder;

use crate::mappers::cursor_mapper::CursorMapper;
use crate::mappers::mapping_error::MappingError;

#[derive(Debug)]
pub struct VisibleStateMapper;

impl VisibleStateMapper {
    pub fn to_domain(dto: &VisibleStateDto) -> Result<VisibleState, MappingError> {
        let mut builder = VisibleStateBuilder::new();
        for raw in &dto.known_refs {
            builder = builder.with_known_ref(MemoryRef::parse(raw.clone())?);
        }
        for raw in &dto.known_dimensions {
            builder = builder.with_known_dimension(DimensionRef::parse(raw.clone())?);
        }
        if let Some(cursor_dto) = dto.active_cursor.as_ref() {
            builder = builder.with_active_cursor(CursorMapper::to_domain(cursor_dto)?);
        }
        builder = builder.with_budget(budget_to_domain(dto.budget));
        Ok(builder.build())
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
