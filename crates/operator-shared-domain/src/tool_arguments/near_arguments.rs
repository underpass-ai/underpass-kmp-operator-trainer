use crate::error::domain_error::DomainError;
use crate::error::domain_result::DomainResult;
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
    ) -> DomainResult<Self> {
        if dimensions.is_empty() {
            return Err(DomainError::EmptyValue {
                context: "near_arguments.dimensions",
            });
        }
        Ok(Self {
            anchor,
            dimensions,
            limit,
        })
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
    fn refuses_empty_dimensions() {
        let anchor = MemoryRef::parse("node:1").unwrap();
        let err = NearArguments::new(anchor, vec![], None).unwrap_err();
        assert!(matches!(
            err,
            DomainError::EmptyValue {
                context: "near_arguments.dimensions"
            }
        ));
    }

    #[test]
    fn limit_is_optional() {
        let anchor = MemoryRef::parse("node:1").unwrap();
        let dim = DimensionRef::parse("temporal").unwrap();
        let args = NearArguments::new(anchor, vec![dim], None).unwrap();
        assert!(args.limit().is_none());
    }
}
