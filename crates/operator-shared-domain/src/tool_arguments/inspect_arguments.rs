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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_target() {
        let target = MemoryRef::parse("node:1").unwrap();
        let args = InspectArguments::new(target.clone());
        assert_eq!(args.target(), &target);
    }
}
