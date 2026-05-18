use crate::value_objects::memory_ref::MemoryRef;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TraceCursor {
    from: MemoryRef,
    to: MemoryRef,
}

impl TraceCursor {
    pub fn new(from: MemoryRef, to: MemoryRef) -> Self {
        Self { from, to }
    }

    pub fn from(&self) -> &MemoryRef {
        &self.from
    }

    pub fn to(&self) -> &MemoryRef {
        &self.to
    }
}
