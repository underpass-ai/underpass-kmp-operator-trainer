use crate::error::domain_result::DomainResult;
use crate::value_objects::dimension_ref::DimensionRef;
use crate::value_objects::memory_ref::MemoryRef;
use crate::value_objects::positive_count::PositiveCount;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NearArguments {
    anchor: MemoryRef,
    dimensions: Vec<DimensionRef>,
    limit: Option<PositiveCount>,
    // The temporal window around the anchor: how many entries to include before
    // and after it. This is the kernel's `kernel_near` window knob and the lever
    // for window expansion — growing `before_entries` reaches back toward a
    // period's start, growing `after_entries` reaches forward toward its end.
    // Both default to 0 (anchor-only).
    before_entries: u32,
    after_entries: u32,
}

impl NearArguments {
    /// `dimensions` is optional (matching the documented `kernel_near` contract
    /// and the kernel itself): an empty list means "all temporal dimensions" —
    /// the natural scope for a single-timeline about, and what the kernel reads
    /// when no dimension selection is sent. A non-empty list scopes the near to
    /// those dimensions.
    pub fn new(
        anchor: MemoryRef,
        dimensions: Vec<DimensionRef>,
        limit: Option<PositiveCount>,
    ) -> DomainResult<Self> {
        Ok(Self {
            anchor,
            dimensions,
            limit,
            before_entries: 0,
            after_entries: 0,
        })
    }

    /// Set the temporal window (entries before/after the anchor). Window
    /// expansion widens this until the period's evidence around the anchor is
    /// covered.
    #[must_use]
    pub fn with_window(mut self, before_entries: u32, after_entries: u32) -> Self {
        self.before_entries = before_entries;
        self.after_entries = after_entries;
        self
    }

    pub fn anchor(&self) -> &MemoryRef {
        &self.anchor
    }

    pub fn dimensions(&self) -> &[DimensionRef] {
        &self.dimensions
    }

    pub fn limit(&self) -> Option<PositiveCount> {
        self.limit
    }

    pub fn before_entries(&self) -> u32 {
        self.before_entries
    }

    pub fn after_entries(&self) -> u32 {
        self.after_entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_anchor_dimensions_and_optional_limit() {
        let anchor = MemoryRef::parse("node:1").unwrap();
        let dim = DimensionRef::parse("temporal").unwrap();
        let args = NearArguments::new(
            anchor.clone(),
            vec![dim.clone()],
            Some(PositiveCount::parse(5, "limit").unwrap()),
        )
        .unwrap();
        assert_eq!(args.anchor(), &anchor);
        assert_eq!(args.dimensions(), &[dim]);
        assert_eq!(args.limit().unwrap().as_usize(), 5);
    }

    #[test]
    fn allows_empty_dimensions_as_an_all_dimension_near() {
        // Empty dimensions means "all temporal dimensions" — the kernel reads
        // every dimension when no selection is sent, and the documented
        // kernel_near contract lists dimensions as optional.
        let anchor = MemoryRef::parse("node:1").unwrap();
        let args = NearArguments::new(anchor, vec![], None).unwrap();
        assert!(args.dimensions().is_empty());
    }

    #[test]
    fn limit_is_optional() {
        let anchor = MemoryRef::parse("node:1").unwrap();
        let dim = DimensionRef::parse("temporal").unwrap();
        let args = NearArguments::new(anchor, vec![dim], None).unwrap();
        assert!(args.limit().is_none());
    }

    #[test]
    fn window_defaults_to_zero_and_is_set_by_with_window() {
        let anchor = MemoryRef::parse("node:1").unwrap();
        let dim = DimensionRef::parse("temporal").unwrap();
        let args = NearArguments::new(anchor, vec![dim], None).unwrap();
        assert_eq!(args.before_entries(), 0);
        assert_eq!(args.after_entries(), 0);

        let widened = args.with_window(4, 6);
        assert_eq!(widened.before_entries(), 4);
        assert_eq!(widened.after_entries(), 6);
    }
}
