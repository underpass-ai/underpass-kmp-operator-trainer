use crate::error::domain_error::DomainError;
use crate::error::domain_result::DomainResult;
use crate::value_objects::dimension_ref::DimensionRef;
use crate::value_objects::memory_ref::MemoryRef;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AroundCursor {
    anchor: MemoryRef,
    dimensions: Vec<DimensionRef>,
}

impl AroundCursor {
    pub fn new(anchor: MemoryRef, dimensions: Vec<DimensionRef>) -> DomainResult<Self> {
        if dimensions.is_empty() {
            return Err(DomainError::EmptyValue {
                context: "around_cursor.dimensions",
            });
        }
        Ok(Self { anchor, dimensions })
    }

    pub fn anchor(&self) -> &MemoryRef {
        &self.anchor
    }

    pub fn dimensions(&self) -> &[DimensionRef] {
        &self.dimensions
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_empty_dimensions() {
        let anchor = MemoryRef::parse("node:1").unwrap();
        assert!(matches!(
            AroundCursor::new(anchor, vec![]),
            Err(DomainError::EmptyValue {
                context: "around_cursor.dimensions"
            })
        ));
    }

    #[test]
    fn keeps_dimensions_in_order() {
        let anchor = MemoryRef::parse("node:1").unwrap();
        let dims = vec![
            DimensionRef::parse("temporal").unwrap(),
            DimensionRef::parse("semantic").unwrap(),
        ];
        let cursor = AroundCursor::new(anchor, dims.clone()).unwrap();
        assert_eq!(cursor.dimensions(), dims.as_slice());
    }
}
