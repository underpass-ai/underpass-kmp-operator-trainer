use crate::value_objects::memory_ref::MemoryRef;
use crate::value_objects::positive_count::PositiveCount;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TraceArguments {
    from: MemoryRef,
    to: Option<MemoryRef>,
    page: PositiveCount,
}

impl TraceArguments {
    pub fn new(from: MemoryRef, to: Option<MemoryRef>, page: PositiveCount) -> Self {
        Self { from, to, page }
    }

    pub fn from(&self) -> &MemoryRef {
        &self.from
    }

    pub fn to(&self) -> Option<&MemoryRef> {
        self.to.as_ref()
    }

    pub fn page(&self) -> PositiveCount {
        self.page
    }
}
