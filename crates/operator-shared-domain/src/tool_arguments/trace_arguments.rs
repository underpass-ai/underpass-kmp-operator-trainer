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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn carries_from_to_and_page() {
        let from = MemoryRef::parse("node:from").unwrap();
        let to = MemoryRef::parse("node:to").unwrap();
        let args = TraceArguments::new(
            from.clone(),
            Some(to.clone()),
            PositiveCount::parse(2, "page").unwrap(),
        );
        assert_eq!(args.from(), &from);
        assert_eq!(args.to(), Some(&to));
        assert_eq!(args.page().as_usize(), 2);
    }

    #[test]
    fn to_is_optional() {
        let from = MemoryRef::parse("node:from").unwrap();
        let args = TraceArguments::new(from, None, PositiveCount::parse(1, "page").unwrap());
        assert!(args.to().is_none());
    }
}
