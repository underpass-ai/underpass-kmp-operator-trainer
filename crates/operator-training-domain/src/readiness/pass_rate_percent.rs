//! Pass rate as a fraction in the closed interval `[0.0, 1.0]`. Used
//! by readiness gates that compare an `EvaluationReport` rate against
//! a target threshold. The value object refuses to be built outside
//! the unit interval or with a non-finite value.

use operator_shared_domain::error::domain_error::DomainError;
use operator_shared_domain::error::domain_result::DomainResult;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct PassRatePercent {
    raw: f64,
}

impl PassRatePercent {
    pub fn parse(value: f64) -> DomainResult<Self> {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(DomainError::UnsupportedValue {
                context: "pass_rate_percent",
                value: format!("{value}"),
            });
        }
        Ok(Self { raw: value })
    }

    pub fn as_f64(self) -> f64 {
        self.raw
    }
}

impl std::fmt::Display for PassRatePercent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:.4}", self.raw)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_negative() {
        assert!(PassRatePercent::parse(-0.1).is_err());
    }

    #[test]
    fn refuses_above_one() {
        assert!(PassRatePercent::parse(1.1).is_err());
    }

    #[test]
    fn refuses_nan() {
        assert!(PassRatePercent::parse(f64::NAN).is_err());
    }

    #[test]
    fn refuses_infinity() {
        assert!(PassRatePercent::parse(f64::INFINITY).is_err());
    }

    #[test]
    #[allow(clippy::float_cmp)] // exact-representable endpoints
    fn accepts_unit_interval_endpoints() {
        assert_eq!(PassRatePercent::parse(0.0).unwrap().as_f64(), 0.0);
        assert_eq!(PassRatePercent::parse(1.0).unwrap().as_f64(), 1.0);
    }
}
