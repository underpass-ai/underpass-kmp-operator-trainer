use crate::value_objects::memory_ref::MemoryRef;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectArguments {
    target: MemoryRef,
}

impl InspectArguments {
    pub fn new(target: MemoryRef) -> Self {
        Self { target }
    }

    pub fn target(&self) -> &MemoryRef {
        &self.target
    }
}
