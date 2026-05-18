//! `VisibleState` is the typed projection of what the Operator perceives
//! when deciding the next action. Mappers in `operator-shared-infra` build
//! this from a wire payload; the domain never sees raw JSON.

use std::collections::BTreeSet;

use crate::cursor::cursor::Cursor;
use crate::value_objects::dimension_ref::DimensionRef;
use crate::value_objects::memory_ref::MemoryRef;
use crate::visible_state::budget_snapshot::BudgetSnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleState {
    known_refs: BTreeSet<MemoryRef>,
    known_dimensions: BTreeSet<DimensionRef>,
    active_cursor: Option<Cursor>,
    budget: BudgetSnapshot,
}

impl VisibleState {
    pub(crate) fn new(
        known_refs: BTreeSet<MemoryRef>,
        known_dimensions: BTreeSet<DimensionRef>,
        active_cursor: Option<Cursor>,
        budget: BudgetSnapshot,
    ) -> Self {
        Self {
            known_refs,
            known_dimensions,
            active_cursor,
            budget,
        }
    }

    /// Public constructor for adapter and integration code. Takes
    /// `IntoIterator` inputs so call sites can pass arrays, vectors or any
    /// other iterable without ceremony.
    ///
    /// The crate-internal `VisibleStateBuilder` is only used by domain
    /// tests; production callers (mappers, replay adapters, runtime
    /// composers) construct typed inputs and pass them here.
    pub fn assemble(
        known_refs: impl IntoIterator<Item = MemoryRef>,
        known_dimensions: impl IntoIterator<Item = DimensionRef>,
        active_cursor: Option<Cursor>,
        budget: BudgetSnapshot,
    ) -> Self {
        Self::new(
            known_refs.into_iter().collect(),
            known_dimensions.into_iter().collect(),
            active_cursor,
            budget,
        )
    }

    pub fn knows_ref(&self, target: &MemoryRef) -> bool {
        self.known_refs.contains(target)
    }

    pub fn knows_dimension(&self, target: &DimensionRef) -> bool {
        self.known_dimensions.contains(target)
    }

    pub fn known_refs(&self) -> &BTreeSet<MemoryRef> {
        &self.known_refs
    }

    pub fn known_dimensions(&self) -> &BTreeSet<DimensionRef> {
        &self.known_dimensions
    }

    pub fn active_cursor(&self) -> Option<&Cursor> {
        self.active_cursor.as_ref()
    }

    pub fn budget(&self) -> BudgetSnapshot {
        self.budget
    }
}
