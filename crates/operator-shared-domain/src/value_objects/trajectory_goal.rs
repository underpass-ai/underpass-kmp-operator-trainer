use crate::error::domain_result::DomainResult;
use crate::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TrajectoryGoal {
    inner: NonEmptyString,
}

impl TrajectoryGoal {
    pub fn parse(value: impl Into<String>) -> DomainResult<Self> {
        Ok(Self {
            inner: NonEmptyString::parse(value, "trajectory_goal")?,
        })
    }

    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }
}

impl std::fmt::Display for TrajectoryGoal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.inner, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::domain_error::DomainError;

    #[test]
    fn accepts_non_empty_goal() {
        let goal =
            TrajectoryGoal::parse("Inspect the current memory node.").expect("goal is non-empty");
        assert_eq!(goal.as_str(), "Inspect the current memory node.");
    }

    #[test]
    fn rejects_blank_goal() {
        assert_eq!(
            TrajectoryGoal::parse("   "),
            Err(DomainError::EmptyValue {
                context: "trajectory_goal"
            })
        );
    }
}
