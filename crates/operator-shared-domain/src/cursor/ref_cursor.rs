use crate::value_objects::memory_ref::MemoryRef;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RefCursor {
    target: MemoryRef,
}

impl RefCursor {
    pub fn new(target: MemoryRef) -> Self {
        Self { target }
    }

    pub fn target(&self) -> &MemoryRef {
        &self.target
    }
}
