use crate::error::domain_error::DomainError;
use crate::error::domain_result::DomainResult;
use crate::value_objects::non_empty_string::NonEmptyString;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestRelationType {
    canonical: NonEmptyString,
}

impl IngestRelationType {
    pub fn parse(value: impl Into<String>) -> DomainResult<Self> {
        let raw = value.into();
        let trimmed = raw.trim();
        let canonical = canonical_relation(trimmed).unwrap_or(trimmed);
        if canonical != trimmed {
            return Err(DomainError::UnsupportedValue {
                context: "ingest.memory.relations[].rel.canonical",
                value: trimmed.to_string(),
            });
        }
        Ok(Self {
            canonical: NonEmptyString::parse(trimmed, "ingest.memory.relations[].rel")?,
        })
    }

    pub fn as_str(&self) -> &str {
        self.canonical.as_str()
    }
}

fn canonical_relation(value: &str) -> Option<&'static str> {
    match normalize_relation(value).as_str() {
        "contains" => Some("contains"),
        "contains_entry" => Some("contains_entry"),
        "has_dimension" => Some("has_dimension"),
        "has_evidence" => Some("has_evidence"),
        "member_of" => Some("member_of"),
        "records" => Some("records"),
        "scoped_to" => Some("scoped_to"),
        "follows" => Some("follows"),
        "answers" => Some("answers"),
        "uses_background" => Some("uses_background"),
        "depends_on" => Some("depends_on"),
        "chosen_because" => Some("chosen_because"),
        "semantic_delta_from" => Some("semantic_delta_from"),
        "updates_state" => Some("updates_state"),
        "supports" => Some("supports"),
        "supports_answer" => Some("supports_answer"),
        "supersedes" => Some("supersedes"),
        "contradicts" | "conflicts" | "conflicts_with" | "conflict_with" => Some("contradicts"),
        "confirms_selection" => Some("confirms_selection"),
        "satisfies_constraint" => Some("satisfies_constraint"),
        "violates_constraint" => Some("violates_constraint"),
        "contributes_to" => Some("contributes_to"),
        "excluded_from" => Some("excluded_from"),
        "checked_against" => Some("checked_against"),
        "derived_from" => Some("derived_from"),
        "restates" => Some("restates"),
        "corrects" => Some("corrects"),
        "component_of" => Some("component_of"),
        "total_of" => Some("total_of"),
        "same_event_as" => Some("same_event_as"),
        "same_entity_as" => Some("same_entity_as"),
        "qualifies_as" => Some("qualifies_as"),
        "matches_requirement" | "matches_question_item" => Some("matches_requirement"),
        _ => None,
    }
}

fn normalize_relation(value: &str) -> String {
    value
        .trim()
        .chars()
        .map(|ch| {
            if ch == '-' || ch == ' ' {
                '_'
            } else {
                ch.to_ascii_lowercase()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_canonical_kernel_relation_values() {
        assert_eq!(
            IngestRelationType::parse("contradicts").unwrap().as_str(),
            "contradicts"
        );
    }

    #[test]
    fn rejects_known_relation_aliases_that_are_not_canonical() {
        assert!(IngestRelationType::parse("conflicts_with").is_err());
    }

    #[test]
    fn accepts_unknown_custom_values_without_normalization() {
        assert_eq!(
            IngestRelationType::parse("org_custom_rel")
                .unwrap()
                .as_str(),
            "org_custom_rel"
        );
    }
}
