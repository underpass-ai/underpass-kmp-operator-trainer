//! Closed generation target vocabulary for synthetic trajectory cases.

use operator_shared_domain::action::operator_action::OperatorAction;
use operator_shared_domain::error::domain_error::DomainError;
use operator_shared_domain::error::domain_result::DomainResult;
use operator_shared_domain::mode::operator_mode::OperatorMode;
use operator_shared_domain::tool::kernel_tool::KernelTool;

use crate::capability::kmp_mcp_capability::KmpMcpCapability;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SyntheticGenerationTarget {
    Kmp(KmpMcpCapability),
    Stop,
    Escalate,
}

impl SyntheticGenerationTarget {
    pub const ALL: [Self; 12] = [
        Self::Kmp(KmpMcpCapability::Ingest),
        Self::Kmp(KmpMcpCapability::Wake),
        Self::Kmp(KmpMcpCapability::Ask),
        Self::Kmp(KmpMcpCapability::Near),
        Self::Kmp(KmpMcpCapability::Goto),
        Self::Kmp(KmpMcpCapability::Rewind),
        Self::Kmp(KmpMcpCapability::Forward),
        Self::Kmp(KmpMcpCapability::Trace),
        Self::Kmp(KmpMcpCapability::Inspect),
        Self::Kmp(KmpMcpCapability::WriteMemory),
        Self::Stop,
        Self::Escalate,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Kmp(capability) => capability.name(),
            Self::Stop => "stop",
            Self::Escalate => "escalate",
        }
    }

    pub fn canonical_mode(self) -> OperatorMode {
        match self {
            Self::Kmp(capability) => capability.mode(),
            Self::Stop | Self::Escalate => OperatorMode::Read,
        }
    }

    pub fn kmp_capability(self) -> Option<KmpMcpCapability> {
        match self {
            Self::Kmp(capability) => Some(capability),
            Self::Stop | Self::Escalate => None,
        }
    }

    pub fn kernel_tool(self) -> Option<KernelTool> {
        self.kmp_capability().map(KmpMcpCapability::tool)
    }

    pub fn parse(value: &str) -> DomainResult<Self> {
        match value {
            "stop" => Ok(Self::Stop),
            "escalate" => Ok(Self::Escalate),
            other => match KernelTool::parse(other) {
                Ok(tool) => Ok(Self::Kmp(KmpMcpCapability::from_tool(tool))),
                Err(DomainError::UnsupportedValue { .. }) => Err(DomainError::UnsupportedValue {
                    context: "synthetic_generation_target",
                    value: value.to_string(),
                }),
                Err(err) => Err(err),
            },
        }
    }

    pub fn from_action(action: &OperatorAction) -> Self {
        match action {
            OperatorAction::ToolCall(_) => Self::Kmp(KmpMcpCapability::from_tool(
                action.tool().expect("tool call has tool"),
            )),
            OperatorAction::Stop(_) => Self::Stop,
            OperatorAction::Escalate(_) => Self::Escalate,
        }
    }

    pub fn matches_action(self, action: &OperatorAction) -> bool {
        match self {
            Self::Kmp(capability) => action.tool() == Some(capability.tool()),
            Self::Stop => matches!(action, OperatorAction::Stop(_)),
            Self::Escalate => matches!(action, OperatorAction::Escalate(_)),
        }
    }
}

impl From<KmpMcpCapability> for SyntheticGenerationTarget {
    fn from(value: KmpMcpCapability) -> Self {
        Self::Kmp(value)
    }
}

impl std::fmt::Display for SyntheticGenerationTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use operator_shared_domain::action::escalate_action::EscalateAction;
    use operator_shared_domain::action::escalate_reason::EscalateReason;
    use operator_shared_domain::action::stop_action::StopAction;
    use operator_shared_domain::action::stop_reason::StopReason;
    use operator_shared_domain::action::tool_call_action::ToolCallAction;
    use operator_shared_domain::tool_arguments::inspect_arguments::InspectArguments;
    use operator_shared_domain::tool_arguments::tool_arguments::ToolArguments;
    use operator_shared_domain::value_objects::memory_ref::MemoryRef;
    use operator_shared_domain::value_objects::model_id::ModelId;

    #[test]
    fn all_covers_kmp_tools_plus_stop_and_escalate() {
        assert_eq!(
            SyntheticGenerationTarget::ALL.len(),
            KmpMcpCapability::ALL.len() + 2
        );
        assert!(SyntheticGenerationTarget::ALL.contains(&SyntheticGenerationTarget::Stop));
        assert!(SyntheticGenerationTarget::ALL.contains(&SyntheticGenerationTarget::Escalate));
    }

    #[test]
    fn kmp_target_exposes_kernel_tool_and_mode() {
        let target = SyntheticGenerationTarget::from(KmpMcpCapability::Inspect);
        assert_eq!(target.name(), "inspect");
        assert_eq!(target.kernel_tool(), Some(KernelTool::Inspect));
        assert_eq!(target.canonical_mode(), OperatorMode::Read);
    }

    #[test]
    fn stop_and_escalate_are_not_kernel_tools() {
        assert_eq!(SyntheticGenerationTarget::Stop.kernel_tool(), None);
        assert_eq!(SyntheticGenerationTarget::Escalate.kernel_tool(), None);
    }

    #[test]
    fn parse_accepts_kmp_tool_names_and_non_tool_targets() {
        assert_eq!(
            SyntheticGenerationTarget::parse("kernel_inspect").unwrap(),
            SyntheticGenerationTarget::from(KmpMcpCapability::Inspect)
        );
        assert_eq!(
            SyntheticGenerationTarget::parse("stop").unwrap(),
            SyntheticGenerationTarget::Stop
        );
        assert!(SyntheticGenerationTarget::parse("kernel_unknown").is_err());
    }

    #[test]
    fn matches_target_action_kind() {
        let inspect = OperatorAction::ToolCall(ToolCallAction::new(ToolArguments::Inspect(
            InspectArguments::new(MemoryRef::parse("node:1").unwrap()),
        )));
        let stop = OperatorAction::Stop(
            StopAction::new(StopReason::AnswerReady, Some("done".to_string()), vec![]).unwrap(),
        );
        let escalate = OperatorAction::Escalate(EscalateAction::new(
            EscalateReason::BeyondCapability,
            ModelId::parse("frontier-reasoner").unwrap(),
        ));

        assert!(
            SyntheticGenerationTarget::from(KmpMcpCapability::Inspect).matches_action(&inspect)
        );
        assert_eq!(
            SyntheticGenerationTarget::from_action(&inspect),
            SyntheticGenerationTarget::from(KmpMcpCapability::Inspect)
        );
        assert!(SyntheticGenerationTarget::Stop.matches_action(&stop));
        assert!(SyntheticGenerationTarget::Escalate.matches_action(&escalate));
        assert!(!SyntheticGenerationTarget::Stop.matches_action(&inspect));
    }
}
