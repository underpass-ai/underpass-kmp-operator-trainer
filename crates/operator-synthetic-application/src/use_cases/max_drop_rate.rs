//! Bounded drop-rate gate for lenient realistic corpus production.

use crate::error::build_realistic_corpus_error::BuildRealisticCorpusError;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct MaxDropRate {
    value: f64,
}

impl MaxDropRate {
    pub fn parse(value: f64) -> Result<Self, BuildRealisticCorpusError> {
        if !value.is_finite() || !(0.0..=1.0).contains(&value) {
            return Err(BuildRealisticCorpusError::InvalidConfiguration {
                message: format!("max_drop_rate must be finite and within [0.0, 1.0], got {value}"),
            });
        }
        Ok(Self { value })
    }

    pub fn as_f64(self) -> f64 {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_bounded_value() {
        assert!((MaxDropRate::parse(0.05).unwrap().as_f64() - 0.05).abs() < f64::EPSILON);
    }

    #[test]
    fn rejects_negative_value() {
        assert!(MaxDropRate::parse(-0.1).is_err());
    }

    #[test]
    fn rejects_value_above_one() {
        assert!(MaxDropRate::parse(1.1).is_err());
    }
}
