//! Builder for `VisibleState`. Mappers in `operator-shared-infra` use this
//! to progressively assemble a `VisibleState` from observations. The
//! builder lives in `domain` because the assembled invariants (such as
//! cursor uniqueness, budget non-negativity) are domain concerns; the data
//! source is the infra concern.

use std::collections::BTreeSet;

use crate::cursor::cursor::Cursor;
use crate::value_objects::dimension_ref::DimensionRef;
use crate::value_objects::memory_ref::MemoryRef;
use crate::visible_state::budget_snapshot::BudgetSnapshot;
use crate::visible_state::visible_state::VisibleState;

#[derive(Debug, Clone)]
pub struct VisibleStateBuilder {
    known_refs: BTreeSet<MemoryRef>,
    known_dimensions: BTreeSet<DimensionRef>,
    active_cursor: Option<Cursor>,
    budget: BudgetSnapshot,
}

impl VisibleStateBuilder {
    pub fn new() -> Self {
        Self {
            known_refs: BTreeSet::new(),
            known_dimensions: BTreeSet::new(),
            active_cursor: None,
            budget: BudgetSnapshot::unbounded(),
        }
    }

    #[must_use]
    pub fn with_known_ref(mut self, target: MemoryRef) -> Self {
        self.known_refs.insert(target);
        self
    }

    #[must_use]
    pub fn with_known_dimension(mut self, target: DimensionRef) -> Self {
        self.known_dimensions.insert(target);
        self
    }

    #[must_use]
    pub fn with_active_cursor(mut self, cursor: Cursor) -> Self {
        self.active_cursor = Some(cursor);
        self
    }

    #[must_use]
    pub fn with_budget(mut self, budget: BudgetSnapshot) -> Self {
        self.budget = budget;
        self
    }

    pub fn build(self) -> VisibleState {
        VisibleState::new(
            self.known_refs,
            self.known_dimensions,
            self.active_cursor,
            self.budget,
        )
    }
}

impl Default for VisibleStateBuilder {
    fn default() -> Self {
        Self::new()
    }
}
