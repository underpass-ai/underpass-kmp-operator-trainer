//! `OperatorMode` is the policy frame Operator is running under. It
//! determines which `KernelTool` variants are allowed for the next action.

use crate::error::domain_error::DomainError;
use crate::error::domain_result::DomainResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum OperatorMode {
    /// Read-only: navigation and inspection tools.
    Read,
    /// Write-only: write memory operations.
    Write,
    /// Full: read and write.
    Full,
    /// Writer's pre-read: a restricted read frame used during a writer
    /// pre-read pass before a write step.
    WriterPreRead,
}

impl OperatorMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Write => "write",
            Self::Full => "full",
            Self::WriterPreRead => "writer_pre_read",
        }
    }

    pub fn parse(value: &str) -> DomainResult<Self> {
        match value {
            "read" => Ok(Self::Read),
            "write" => Ok(Self::Write),
            "full" => Ok(Self::Full),
            "writer_pre_read" => Ok(Self::WriterPreRead),
            other => Err(DomainError::UnsupportedValue {
                context: "operator_mode",
                value: other.to_string(),
            }),
        }
    }
}

impl std::fmt::Display for OperatorMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_each_variant() {
        for mode in [
            OperatorMode::Read,
            OperatorMode::Write,
            OperatorMode::Full,
            OperatorMode::WriterPreRead,
        ] {
            assert_eq!(OperatorMode::parse(mode.as_str()).unwrap(), mode);
        }
    }

    #[test]
    fn refuses_unknown_mode() {
        assert!(matches!(
            OperatorMode::parse("invalid"),
            Err(DomainError::UnsupportedValue {
                context: "operator_mode",
                ..
            })
        ));
    }
}
