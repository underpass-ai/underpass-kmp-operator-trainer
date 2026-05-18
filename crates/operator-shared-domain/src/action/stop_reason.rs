use crate::error::domain_error::DomainError;
use crate::error::domain_result::DomainResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum StopReason {
    AnswerReady,
    NoCandidate,
    BudgetExhausted,
}

impl StopReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AnswerReady => "answer_ready",
            Self::NoCandidate => "no_candidate",
            Self::BudgetExhausted => "budget_exhausted",
        }
    }

    pub fn parse(value: &str) -> DomainResult<Self> {
        match value {
            "answer_ready" => Ok(Self::AnswerReady),
            "no_candidate" => Ok(Self::NoCandidate),
            "budget_exhausted" => Ok(Self::BudgetExhausted),
            other => Err(DomainError::UnsupportedValue {
                context: "stop_reason",
                value: other.to_string(),
            }),
        }
    }
}

impl std::fmt::Display for StopReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}
