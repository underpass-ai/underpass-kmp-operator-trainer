//! Ratio in the closed interval `[0.0, 1.0]` for split policies.

use operator_shared_domain::error::domain_error::DomainError;

use crate::error::synthetic_domain_result::SyntheticDomainResult;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct BoundedRatio {
    raw: f64,
}

impl BoundedRatio {
    pub fn parse(value: f64) -> SyntheticDomainResult<Self> {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(DomainError::UnsupportedValue {
                context: "bounded_ratio",
                value: format!("{value}"),
            }
            .into());
        }
        Ok(Self { raw: value })
    }

    pub fn as_f64(self) -> f64 {
        self.raw
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_unit_interval_values() {
        assert_eq!(
            BoundedRatio::parse(0.0).unwrap().as_f64().to_bits(),
            0.0f64.to_bits()
        );
        assert_eq!(
            BoundedRatio::parse(1.0).unwrap().as_f64().to_bits(),
            1.0f64.to_bits()
        );
    }

    #[test]
    fn rejects_values_outside_unit_interval() {
        assert!(BoundedRatio::parse(-0.1).is_err());
        assert!(BoundedRatio::parse(1.1).is_err());
        assert!(BoundedRatio::parse(f64::NAN).is_err());
    }
}
