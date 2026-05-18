use crate::value_objects::dimension_ref::DimensionRef;
use crate::value_objects::memory_ref::MemoryRef;
use crate::value_objects::positive_count::PositiveCount;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NearArguments {
    anchor: MemoryRef,
    dimensions: Vec<DimensionRef>,
    limit: Option<PositiveCount>,
}

impl NearArguments {
    pub fn new(
        anchor: MemoryRef,
        dimensions: Vec<DimensionRef>,
        limit: Option<PositiveCount>,
    ) -> Self {
        Self {
            anchor,
            dimensions,
            limit,
        }
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
}
