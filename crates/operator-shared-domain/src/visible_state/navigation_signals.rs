//! `NavigationSignals` is the per-step bundle of intrinsic (no-gold) signals
//! the operator reads from a KMP/MCP read response to estimate how far the
//! retrieved context is from the still-needed context. The infra executor
//! parses these from the MCP JSON (`coverage.dimensions`, `proof.frontier_size`,
//! `proof.conflicts`, `proof.confidence`, `page`, `quality`, inspect links) and
//! the runtime loop feeds them into [`super::context_coverage_deviation::ContextCoverageDeviation`].
//!
//! These are kernel self-reports, not gold labels: they are computed at run
//! time with no knowledge of the answer.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NavigationSignals {
    coverage_requested: u32,
    coverage_missing: u32,
    page_has_more: bool,
    page_returned: u32,
    page_total: u32,
    frontier_size: u32,
    conflicts: u32,
    confidence_is_unknown: bool,
    answer_is_null: bool,
    unvisited_links: u32,
    total_links: u32,
}

impl NavigationSignals {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        coverage_requested: u32,
        coverage_missing: u32,
        page_has_more: bool,
        page_returned: u32,
        page_total: u32,
        frontier_size: u32,
        conflicts: u32,
        confidence_is_unknown: bool,
        answer_is_null: bool,
        unvisited_links: u32,
        total_links: u32,
    ) -> Self {
        Self {
            coverage_requested,
            coverage_missing,
            page_has_more,
            page_returned,
            page_total,
            frontier_size,
            conflicts,
            confidence_is_unknown,
            answer_is_null,
            unvisited_links,
            total_links,
        }
    }

    /// Neutral signals (nothing missing, no frontier, answer present): a
    /// response that contributes no deviation. Used by fixtures and as the
    /// fallback when a backend returns no coverage metadata.
    pub fn empty() -> Self {
        Self::new(0, 0, false, 0, 0, 0, 0, false, false, 0, 0)
    }

    pub fn coverage_requested(&self) -> u32 {
        self.coverage_requested
    }

    pub fn coverage_missing(&self) -> u32 {
        self.coverage_missing
    }

    pub fn page_has_more(&self) -> bool {
        self.page_has_more
    }

    pub fn page_returned(&self) -> u32 {
        self.page_returned
    }

    pub fn page_total(&self) -> u32 {
        self.page_total
    }

    pub fn frontier_size(&self) -> u32 {
        self.frontier_size
    }

    pub fn conflicts(&self) -> u32 {
        self.conflicts
    }

    pub fn confidence_is_unknown(&self) -> bool {
        self.confidence_is_unknown
    }

    pub fn answer_is_null(&self) -> bool {
        self.answer_is_null
    }

    pub fn unvisited_links(&self) -> u32 {
        self.unvisited_links
    }

    pub fn total_links(&self) -> u32 {
        self.total_links
    }
}
