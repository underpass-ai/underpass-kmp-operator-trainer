use crate::value_objects::dimension_ref::DimensionRef;
use crate::value_objects::memory_ref::MemoryRef;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRef {
    target: MemoryRef,
    dimensions: Vec<DimensionRef>,
}

impl EvidenceRef {
    pub fn new(target: MemoryRef, dimensions: Vec<DimensionRef>) -> Self {
        Self { target, dimensions }
    }

    pub fn target(&self) -> &MemoryRef {
        &self.target
    }

    pub fn dimensions(&self) -> &[DimensionRef] {
        &self.dimensions
    }
}
