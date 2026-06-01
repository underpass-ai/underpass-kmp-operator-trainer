//! `VisibleState` is the typed projection of what the Operator perceives
//! when deciding the next action. Mappers in `operator-shared-infra` build
//! this from a wire payload; the domain never sees raw JSON.

use std::collections::BTreeSet;

use crate::cursor::cursor::Cursor;
use crate::ids::about_id::AboutId;
use crate::value_objects::dimension_ref::DimensionRef;
use crate::value_objects::memory_ref::MemoryRef;
use crate::visible_state::budget_snapshot::BudgetSnapshot;
use crate::visible_state::coverage_deviation_snapshot::CoverageDeviationSnapshot;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VisibleState {
    known_refs: BTreeSet<MemoryRef>,
    known_dimensions: BTreeSet<DimensionRef>,
    active_cursor: Option<Cursor>,
    budget: BudgetSnapshot,
    coverage_deviation: CoverageDeviationSnapshot,
    // Candidate sessions (abouts) the operator should cover for a cross-about
    // goal — what retrieval surfaced. Defaults empty; set via
    // `with_candidate_abouts`. Preserved across `observing` so it stays in the
    // operator's context for the whole session.
    candidate_abouts: Vec<AboutId>,
}

impl VisibleState {
    pub(crate) fn new(
        known_refs: BTreeSet<MemoryRef>,
        known_dimensions: BTreeSet<DimensionRef>,
        active_cursor: Option<Cursor>,
        budget: BudgetSnapshot,
        coverage_deviation: CoverageDeviationSnapshot,
    ) -> Self {
        Self {
            known_refs,
            known_dimensions,
            active_cursor,
            budget,
            coverage_deviation,
            candidate_abouts: Vec::new(),
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
            CoverageDeviationSnapshot::unknown(),
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

    pub fn coverage_deviation(&self) -> CoverageDeviationSnapshot {
        self.coverage_deviation
    }

    /// The candidate sessions (abouts) the operator should cover, as surfaced by
    /// retrieval. Empty for single-about sessions.
    pub fn candidate_abouts(&self) -> &[AboutId] {
        &self.candidate_abouts
    }

    /// Seed the candidate sessions the operator must cover for a cross-about
    /// goal. Used by the cross-about compiler so the entry state carries the
    /// session set the operator should wake.
    #[must_use]
    pub fn with_candidate_abouts(mut self, candidate_abouts: Vec<AboutId>) -> Self {
        self.candidate_abouts = candidate_abouts;
        self
    }

    /// Override the coverage-deviation snapshot. Used by the wire mapper to
    /// thread a deserialized deviation into an assembled state.
    #[must_use]
    pub fn with_coverage_deviation(
        mut self,
        coverage_deviation: CoverageDeviationSnapshot,
    ) -> Self {
        self.coverage_deviation = coverage_deviation;
        self
    }

    /// Returns the next visible state after a tool observation: the operator now
    /// also knows `observed_refs`, its remaining `budget` reflects the call just
    /// consumed, and `coverage_deviation` is the updated context-coverage
    /// estimate. The runtime multi-step loop calls this to feed each observation
    /// back into the state the policy perceives on the next step. Known
    /// dimensions and the active cursor are preserved.
    #[must_use]
    pub fn observing(
        &self,
        observed_refs: impl IntoIterator<Item = MemoryRef>,
        budget: BudgetSnapshot,
        coverage_deviation: CoverageDeviationSnapshot,
    ) -> Self {
        let mut known_refs = self.known_refs.clone();
        known_refs.extend(observed_refs);
        Self {
            known_refs,
            known_dimensions: self.known_dimensions.clone(),
            active_cursor: self.active_cursor.clone(),
            budget,
            coverage_deviation,
            candidate_abouts: self.candidate_abouts.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn memory_ref(value: &str) -> MemoryRef {
        MemoryRef::parse(value).expect("valid memory ref")
    }

    #[test]
    fn observing_adds_new_refs_and_updates_budget_keeping_dimensions_and_cursor() {
        let state = VisibleState::assemble(
            [memory_ref("node:1")],
            [],
            None,
            BudgetSnapshot::bounded(3, 4096),
        );

        let next = state.observing(
            [memory_ref("node:2"), memory_ref("node:3")],
            BudgetSnapshot::bounded(2, 4096),
            CoverageDeviationSnapshot::new(350, false, true),
        );

        assert!(next.knows_ref(&memory_ref("node:1")));
        assert!(next.knows_ref(&memory_ref("node:2")));
        assert!(next.knows_ref(&memory_ref("node:3")));
        assert_eq!(next.known_refs().len(), 3);
        assert_eq!(next.known_dimensions(), state.known_dimensions());
        assert_eq!(next.active_cursor(), state.active_cursor());
        assert_eq!(
            next.budget().calls_remaining(),
            BudgetSnapshot::bounded(2, 4096).calls_remaining()
        );
        assert_eq!(next.coverage_deviation().deviation_milli(), 350);
        assert!(next.coverage_deviation().conflict_blocking());
    }

    #[test]
    fn observing_with_no_refs_only_updates_budget() {
        let state = VisibleState::assemble(
            [memory_ref("node:1")],
            [],
            None,
            BudgetSnapshot::bounded(2, 4096),
        );

        let next = state.observing(
            std::iter::empty(),
            BudgetSnapshot::bounded(1, 4096),
            CoverageDeviationSnapshot::unknown(),
        );

        assert_eq!(next.known_refs(), state.known_refs());
        assert!(next.budget().allows_another_call());
    }
}
