use std::collections::BTreeSet;

use crate::action::operator_action::OperatorAction;
use crate::contract::action_contract_subject::ActionContractSubject;
use crate::contract::contract_violation::ContractViolation;
use crate::contract::contract_violation_code::ContractViolationCode;
use crate::specifications::specification::Specification;
use crate::tool_arguments::ingest_arguments::IngestArguments;
use crate::tool_arguments::tool_arguments::ToolArguments;
use crate::value_objects::memory_ref::MemoryRef;

#[derive(Debug, Default)]
pub struct IngestEvidenceSupportsKnownRefsSpec;

impl IngestEvidenceSupportsKnownRefsSpec {
    pub fn new() -> Self {
        Self
    }
}

impl Specification<ActionContractSubject<'_>> for IngestEvidenceSupportsKnownRefsSpec {
    fn evaluate(&self, subject: &ActionContractSubject<'_>) -> Result<(), ContractViolation> {
        let OperatorAction::ToolCall(call) = subject.action() else {
            return Ok(());
        };
        let ToolArguments::Ingest(args) = call.arguments() else {
            return Ok(());
        };
        check_ingest_evidence_supports(args, subject)
    }
}

fn check_ingest_evidence_supports(
    args: &IngestArguments,
    subject: &ActionContractSubject<'_>,
) -> Result<(), ContractViolation> {
    let known = known_refs(args, subject);
    for evidence in args.memory().evidence() {
        for support in evidence.supports() {
            if !known.contains(support) {
                return Err(ContractViolation::new(
                    ContractViolationCode::UnknownMemoryRef,
                    "ingest.memory.evidence[].supports",
                    format!("evidence support '{support}' is not declared or visible"),
                ));
            }
        }
    }
    Ok(())
}

fn known_refs(args: &IngestArguments, subject: &ActionContractSubject<'_>) -> BTreeSet<MemoryRef> {
    let mut known = subject.visible().known_refs().clone();
    known.extend(
        args.memory()
            .entries()
            .iter()
            .map(|entry| entry.id().clone()),
    );
    known
}
