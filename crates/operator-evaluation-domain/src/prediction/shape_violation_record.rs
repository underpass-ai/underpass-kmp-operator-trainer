//! Recoverable prediction shape violation found while reading
//! `predictions.jsonl`. These rows are still part of the evaluation
//! denominator, but they cannot become typed `OperatorAction` values.

use operator_shared_domain::error::domain_result::DomainResult;
use operator_shared_domain::ids::step_id::StepId;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;
use operator_shared_domain::value_objects::positive_count::PositiveCount;

use crate::error::evaluation_domain_result::EvaluationDomainResult;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShapeViolationRecord {
    line: PositiveCount,
    step_id: Option<StepId>,
    message: NonEmptyString,
}

impl ShapeViolationRecord {
    pub fn new(
        line: usize,
        step_id: Option<StepId>,
        message: impl Into<String>,
    ) -> EvaluationDomainResult<Self> {
        Ok(Self {
            line: parse_line(line)?,
            step_id,
            message: NonEmptyString::parse(message, "shape_violation.message")?,
        })
    }

    pub fn line(&self) -> usize {
        self.line.as_usize()
    }

    pub fn step_id(&self) -> Option<&StepId> {
        self.step_id.as_ref()
    }

    pub fn message(&self) -> &str {
        self.message.as_str()
    }
}

fn parse_line(line: usize) -> DomainResult<PositiveCount> {
    PositiveCount::parse(line, "shape_violation.line")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_positive_line_and_message() {
        let record = ShapeViolationRecord::new(7, None, "bad action").unwrap();

        assert_eq!(record.line(), 7);
        assert_eq!(record.step_id(), None);
        assert_eq!(record.message(), "bad action");
    }

    #[test]
    fn rejects_zero_line() {
        assert!(ShapeViolationRecord::new(0, None, "bad action").is_err());
    }

    #[test]
    fn rejects_empty_message() {
        assert!(ShapeViolationRecord::new(1, None, " ").is_err());
    }
}
