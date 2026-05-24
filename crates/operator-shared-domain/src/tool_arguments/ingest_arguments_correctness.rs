use crate::contract::correctness::action_correctness::ActionCorrectness;
use crate::contract::correctness::action_correctness_outcome::ActionCorrectnessOutcome;
use crate::contract::correctness::correctness_mode::CorrectnessMode;
use crate::contract::correctness::field_outcome::FieldOutcome;
use crate::contract::correctness::field_path::FieldPath;
use crate::contract::correctness::field_result::FieldResult;
use crate::contract::correctness::field_result_helpers::{
    field_result_exact, field_result_exact_bool, field_result_exact_debug,
    field_result_schema_valid_non_empty, field_result_schema_valid_optional_non_empty,
    field_result_schema_valid_system_time,
};
use crate::tool_arguments::ingest_arguments::IngestArguments;
use crate::tool_arguments::ingest_provenance::IngestProvenance;

impl ActionCorrectness for IngestArguments {
    fn evaluate_correctness(&self, ground_truth: &Self) -> ActionCorrectnessOutcome {
        let mut results = vec![
            field_result_exact(
                "about",
                self.about().as_str().to_string(),
                ground_truth.about().as_str().to_string(),
            ),
            field_result_exact_bool("dry_run", self.dry_run(), ground_truth.dry_run()),
            // The current domain contract types this as NonEmptyString. It
            // is generated/idempotent content, so correctness is schema
            // validity, not equality with the ground-truth literal.
            field_result_schema_valid_non_empty("idempotency_key", self.idempotency_key()),
            field_result_exact_debug(
                "memory.dimensions[*]",
                &self.memory().dimensions(),
                &ground_truth.memory().dimensions(),
            ),
            field_result_exact_debug(
                "memory.entries[*]",
                &self.memory().entries(),
                &ground_truth.memory().entries(),
            ),
            field_result_exact_debug(
                "memory.relations[*]",
                &self.memory().relations(),
                &ground_truth.memory().relations(),
            ),
            field_result_exact_debug(
                "memory.evidence[*]",
                &self.memory().evidence(),
                &ground_truth.memory().evidence(),
            ),
        ];
        results.extend(provenance_results(
            self.provenance(),
            ground_truth.provenance(),
        ));
        ActionCorrectnessOutcome::new(results)
    }
}

fn provenance_results(
    actual: Option<&IngestProvenance>,
    expected: Option<&IngestProvenance>,
) -> Vec<FieldResult> {
    match (actual, expected) {
        (None, None) => vec![FieldResult::new(
            FieldPath::trusted_static("provenance"),
            CorrectnessMode::SchemaValid,
            FieldOutcome::Pass,
        )],
        (None, Some(_)) => vec![FieldResult::new(
            FieldPath::trusted_static("provenance"),
            CorrectnessMode::SchemaValid,
            FieldOutcome::SchemaInvalid {
                message: "expected provenance to be present".to_string(),
            },
        )],
        (Some(_actual), None) => vec![FieldResult::new(
            FieldPath::trusted_static("provenance"),
            CorrectnessMode::SchemaValid,
            FieldOutcome::Pass,
        )],
        (Some(actual), Some(expected)) => vec![
            field_result_exact(
                "provenance.source_kind",
                actual.source_kind().as_str().to_string(),
                expected.source_kind().as_str().to_string(),
            ),
            field_result_schema_valid_non_empty("provenance.source_agent", actual.source_agent()),
            field_result_schema_valid_system_time(
                "provenance.observed_at",
                Some(actual.observed_at()),
                Some(expected.observed_at()),
            ),
            field_result_schema_valid_optional_non_empty(
                "provenance.correlation_id",
                actual.correlation_id(),
                expected.correlation_id(),
            ),
            field_result_schema_valid_optional_non_empty(
                "provenance.causation_id",
                actual.causation_id(),
                expected.causation_id(),
            ),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::UNIX_EPOCH;

    use crate::ids::about_id::AboutId;
    use crate::tool_arguments::ingest_dimension::IngestDimension;
    use crate::tool_arguments::ingest_entry::IngestEntry;
    use crate::tool_arguments::ingest_memory::IngestMemory;
    use crate::tool_arguments::ingest_provenance::IngestProvenance;
    use crate::tool_arguments::ingest_source_kind::IngestSourceKind;
    use crate::tool_arguments::ingest_temporal_coordinate::IngestTemporalCoordinate;
    use crate::value_objects::dimension_ref::DimensionRef;
    use crate::value_objects::memory_ref::MemoryRef;
    use crate::value_objects::non_empty_string::NonEmptyString;
    use crate::value_objects::positive_count::PositiveCount;
    use crate::value_objects::string_map::StringMap;

    fn text(value: &str) -> NonEmptyString {
        NonEmptyString::parse(value, "test").unwrap()
    }

    fn args(idempotency_key: &str, observed_offset_secs: u64) -> IngestArguments {
        let dimension = IngestDimension::new(
            DimensionRef::parse("agent:writer").unwrap(),
            text("agent"),
            Some(text("Writer")),
            StringMap::empty(),
        );
        let coordinate = IngestTemporalCoordinate::new(
            DimensionRef::parse("agent:writer").unwrap(),
            text("about:test"),
            None,
            None,
            None,
            None,
            None,
            Some(PositiveCount::parse(1, "sequence").unwrap()),
            None,
            StringMap::empty(),
        )
        .unwrap();
        let entry = IngestEntry::new(
            MemoryRef::parse("entry:1").unwrap(),
            text("decision"),
            text("Prepared ingest."),
            vec![coordinate],
            StringMap::empty(),
        )
        .unwrap();
        let memory = IngestMemory::new(vec![dimension], vec![entry], vec![], vec![]).unwrap();
        let provenance = IngestProvenance::new(
            IngestSourceKind::Agent,
            text("builder"),
            UNIX_EPOCH + std::time::Duration::from_secs(observed_offset_secs),
            Some(text("corr:any")),
            Some(text("cause:any")),
        );
        IngestArguments::new(
            AboutId::parse("about:test").unwrap(),
            memory,
            Some(provenance),
            text(idempotency_key),
            true,
        )
    }

    #[test]
    fn generated_non_empty_fields_are_schema_valid_not_exact() {
        let actual = args("idem:actual", 10);
        let expected = args("idem:expected", 20);

        assert!(actual.evaluate_correctness(&expected).is_correct());
    }

    #[test]
    fn structural_memory_fields_remain_exact() {
        let actual = args("idem:actual", 10);
        let mut expected = args("idem:expected", 20);
        expected = IngestArguments::new(
            expected.about().clone(),
            IngestMemory::new(vec![], expected.memory().entries().to_vec(), vec![], vec![])
                .unwrap(),
            expected.provenance().cloned(),
            expected.idempotency_key().clone(),
            expected.dry_run(),
        );

        assert!(!actual.evaluate_correctness(&expected).is_correct());
    }
}
