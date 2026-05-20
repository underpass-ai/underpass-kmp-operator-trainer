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
pub struct IngestRelationRefsKnownSpec;

impl IngestRelationRefsKnownSpec {
    pub fn new() -> Self {
        Self
    }
}

impl Specification<ActionContractSubject<'_>> for IngestRelationRefsKnownSpec {
    fn evaluate(&self, subject: &ActionContractSubject<'_>) -> Result<(), ContractViolation> {
        let OperatorAction::ToolCall(call) = subject.action() else {
            return Ok(());
        };
        let ToolArguments::Ingest(args) = call.arguments() else {
            return Ok(());
        };
        check_ingest_relation_refs(args, subject)
    }
}

fn check_ingest_relation_refs(
    args: &IngestArguments,
    subject: &ActionContractSubject<'_>,
) -> Result<(), ContractViolation> {
    let known = known_refs(args, subject);
    for relation in args.memory().relations() {
        check_ref(
            &known,
            relation.source_ref(),
            "ingest.memory.relations[].from",
        )?;
        check_ref(
            &known,
            relation.target_ref(),
            "ingest.memory.relations[].to",
        )?;
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

fn check_ref(
    known: &BTreeSet<MemoryRef>,
    target: &MemoryRef,
    field: &str,
) -> Result<(), ContractViolation> {
    if known.contains(target) {
        return Ok(());
    }
    Err(ContractViolation::new(
        ContractViolationCode::UnknownMemoryRef,
        field,
        format!("memory ref '{target}' is not declared in ingest action or visible state"),
    ))
}
