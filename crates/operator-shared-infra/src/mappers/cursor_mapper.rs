use operator_shared_contract::cursor_dto::CursorDto;
use operator_shared_domain::cursor::around_cursor::AroundCursor;
use operator_shared_domain::cursor::cursor::Cursor;
use operator_shared_domain::cursor::ref_cursor::RefCursor;
use operator_shared_domain::cursor::temporal_anchor::TemporalAnchor;
use operator_shared_domain::cursor::temporal_cursor::TemporalCursor;
use operator_shared_domain::cursor::temporal_cursor_key::TemporalCursorKey;
use operator_shared_domain::cursor::trace_cursor::TraceCursor;
use operator_shared_domain::value_objects::dimension_ref::DimensionRef;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;

use crate::mappers::mapping_error::MappingError;

#[derive(Debug)]
pub struct CursorMapper;

impl CursorMapper {
    pub fn to_domain(dto: &CursorDto) -> Result<Cursor, MappingError> {
        match dto {
            CursorDto::Ref { target } => Ok(Cursor::Ref(RefCursor::new(MemoryRef::parse(target)?))),
            CursorDto::Around { anchor, dimensions } => {
                let anchor = MemoryRef::parse(anchor)?;
                let mut typed_dims = Vec::with_capacity(dimensions.len());
                for dim in dimensions {
                    typed_dims.push(DimensionRef::parse(dim)?);
                }
                Ok(Cursor::Around(AroundCursor::new(anchor, typed_dims)?))
            }
            CursorDto::Temporal { key, anchor } => {
                let key = TemporalCursorKey::parse(key)?;
                let anchor = TemporalAnchor::parse(anchor)?;
                Ok(Cursor::Temporal(TemporalCursor::new(key, anchor)))
            }
            CursorDto::Trace { from, to } => {
                let from = MemoryRef::parse(from)?;
                let to = MemoryRef::parse(to)?;
                Ok(Cursor::Trace(TraceCursor::new(from, to)))
            }
        }
    }

    pub fn to_dto(domain: &Cursor) -> CursorDto {
        match domain {
            Cursor::Ref(rc) => CursorDto::Ref {
                target: rc.target().as_str().to_string(),
            },
            Cursor::Around(ac) => CursorDto::Around {
                anchor: ac.anchor().as_str().to_string(),
                dimensions: ac
                    .dimensions()
                    .iter()
                    .map(|d| d.as_str().to_string())
                    .collect(),
            },
            Cursor::Temporal(tc) => CursorDto::Temporal {
                key: tc.key().as_str().to_string(),
                anchor: tc.anchor().as_str().to_string(),
            },
            Cursor::Trace(tc) => CursorDto::Trace {
                from: tc.from().as_str().to_string(),
                to: tc.to().as_str().to_string(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_around_cursor() {
        let dto = CursorDto::Around {
            anchor: "node:1".to_string(),
            dimensions: vec!["temporal".to_string()],
        };
        let domain = CursorMapper::to_domain(&dto).expect("valid around cursor");
        let re_dto = CursorMapper::to_dto(&domain);
        assert_eq!(dto, re_dto);
    }

    #[test]
    fn refuses_empty_dimensions_for_around_cursor() {
        let dto = CursorDto::Around {
            anchor: "node:1".to_string(),
            dimensions: vec![],
        };
        let err = CursorMapper::to_domain(&dto).expect_err("empty dimensions must fail");
        assert!(matches!(err, MappingError::Domain(_)));
    }
}
