use std::collections::BTreeSet;

use crate::action::operator_action::OperatorAction;
use crate::contract::action_contract_subject::ActionContractSubject;
use crate::contract::contract_violation::ContractViolation;
use crate::contract::contract_violation_code::ContractViolationCode;
use crate::specifications::specification::Specification;
use crate::tool_arguments::ingest_arguments::IngestArguments;
use crate::tool_arguments::tool_arguments::ToolArguments;
use crate::value_objects::dimension_ref::DimensionRef;

#[derive(Debug, Default)]
pub struct IngestCoordinateDimensionsKnownSpec;

impl IngestCoordinateDimensionsKnownSpec {
    pub fn new() -> Self {
        Self
    }
}

impl Specification<ActionContractSubject<'_>> for IngestCoordinateDimensionsKnownSpec {
    fn evaluate(&self, subject: &ActionContractSubject<'_>) -> Result<(), ContractViolation> {
        let OperatorAction::ToolCall(call) = subject.action() else {
            return Ok(());
        };
        let ToolArguments::Ingest(args) = call.arguments() else {
            return Ok(());
        };
        check_ingest_dimensions(args, subject)
    }
}

fn check_ingest_dimensions(
    args: &IngestArguments,
    subject: &ActionContractSubject<'_>,
) -> Result<(), ContractViolation> {
    let mut known: BTreeSet<DimensionRef> = subject.visible().known_dimensions().clone();
    known.extend(
        args.memory()
            .dimensions()
            .iter()
            .map(|dim| dim.id().clone()),
    );
    for entry in args.memory().entries() {
        for coordinate in entry.coordinates() {
            if !known.contains(coordinate.dimension()) {
                return Err(ContractViolation::new(
                    ContractViolationCode::UnknownDimension,
                    "ingest.memory.entries[].coordinates[].dimension",
                    format!(
                        "dimension '{}' is not declared or visible",
                        coordinate.dimension()
                    ),
                ));
            }
        }
    }
    Ok(())
}
