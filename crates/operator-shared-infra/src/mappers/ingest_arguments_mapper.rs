use std::collections::BTreeMap;
use std::time::SystemTime;

use operator_shared_contract::per_tool::ingest_arguments_dto::IngestArgumentsDto;
use operator_shared_contract::per_tool::ingest_dimension_dto::IngestDimensionDto;
use operator_shared_contract::per_tool::ingest_entry_dto::IngestEntryDto;
use operator_shared_contract::per_tool::ingest_evidence_dto::IngestEvidenceDto;
use operator_shared_contract::per_tool::ingest_memory_dto::IngestMemoryDto;
use operator_shared_contract::per_tool::ingest_provenance_dto::IngestProvenanceDto;
use operator_shared_contract::per_tool::ingest_relation_dto::IngestRelationDto;
use operator_shared_contract::per_tool::ingest_temporal_coordinate_dto::IngestTemporalCoordinateDto;
use operator_shared_domain::ids::about_id::AboutId;
use operator_shared_domain::tool_arguments::ingest_arguments::IngestArguments;
use operator_shared_domain::tool_arguments::ingest_confidence::IngestConfidence;
use operator_shared_domain::tool_arguments::ingest_dimension::IngestDimension;
use operator_shared_domain::tool_arguments::ingest_entry::IngestEntry;
use operator_shared_domain::tool_arguments::ingest_evidence::IngestEvidence;
use operator_shared_domain::tool_arguments::ingest_memory::IngestMemory;
use operator_shared_domain::tool_arguments::ingest_provenance::IngestProvenance;
use operator_shared_domain::tool_arguments::ingest_relation::IngestRelation;
use operator_shared_domain::tool_arguments::ingest_relation_class::IngestRelationClass;
use operator_shared_domain::tool_arguments::ingest_relation_type::IngestRelationType;
use operator_shared_domain::tool_arguments::ingest_source_kind::IngestSourceKind;
use operator_shared_domain::tool_arguments::ingest_temporal_coordinate::IngestTemporalCoordinate;
use operator_shared_domain::value_objects::dimension_ref::DimensionRef;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::non_empty_string::NonEmptyString;
use operator_shared_domain::value_objects::positive_count::PositiveCount;
use operator_shared_domain::value_objects::string_map::StringMap;

use crate::mappers::mapping_error::MappingError;

#[derive(Debug)]
pub struct IngestArgumentsMapper;

impl IngestArgumentsMapper {
    pub fn to_domain(dto: IngestArgumentsDto) -> Result<IngestArguments, MappingError> {
        Ok(IngestArguments::new(
            AboutId::parse(dto.about)?,
            ingest_memory_to_domain(dto.memory)?,
            dto.provenance
                .map(ingest_provenance_to_domain)
                .transpose()?,
            NonEmptyString::parse(dto.idempotency_key, "ingest.idempotency_key")?,
            dto.dry_run.unwrap_or(false),
        ))
    }

    pub fn to_dto(domain: &IngestArguments) -> IngestArgumentsDto {
        IngestArgumentsDto {
            about: domain.about().as_str().to_string(),
            memory: ingest_memory_to_dto(domain.memory()),
            provenance: domain.provenance().map(ingest_provenance_to_dto),
            idempotency_key: domain.idempotency_key().as_str().to_string(),
            dry_run: Some(domain.dry_run()),
        }
    }
}

fn ingest_memory_to_domain(dto: IngestMemoryDto) -> Result<IngestMemory, MappingError> {
    let mut dimensions = Vec::with_capacity(dto.dimensions.len());
    for dimension in dto.dimensions {
        dimensions.push(ingest_dimension_to_domain(dimension)?);
    }
    let mut entries = Vec::with_capacity(dto.entries.len());
    for entry in dto.entries {
        entries.push(ingest_entry_to_domain(entry)?);
    }
    let mut relations = Vec::with_capacity(dto.relations.len());
    for relation in dto.relations {
        relations.push(ingest_relation_to_domain(relation)?);
    }
    let mut evidence = Vec::with_capacity(dto.evidence.len());
    for item in dto.evidence {
        evidence.push(ingest_evidence_to_domain(item)?);
    }
    Ok(IngestMemory::new(dimensions, entries, relations, evidence)?)
}

fn ingest_dimension_to_domain(dto: IngestDimensionDto) -> Result<IngestDimension, MappingError> {
    Ok(IngestDimension::new(
        DimensionRef::parse(dto.id)?,
        NonEmptyString::parse(dto.kind, "ingest.memory.dimensions[].kind")?,
        dto.title
            .map(|title| NonEmptyString::parse(title, "ingest.memory.dimensions[].title"))
            .transpose()?,
        string_map_to_domain(dto.metadata)?,
    ))
}

fn ingest_entry_to_domain(dto: IngestEntryDto) -> Result<IngestEntry, MappingError> {
    let mut coordinates = Vec::with_capacity(dto.coordinates.len());
    for coordinate in dto.coordinates {
        coordinates.push(ingest_coordinate_to_domain(coordinate)?);
    }
    Ok(IngestEntry::new(
        MemoryRef::parse(dto.id)?,
        NonEmptyString::parse(dto.kind, "ingest.memory.entries[].kind")?,
        NonEmptyString::parse(dto.text, "ingest.memory.entries[].text")?,
        coordinates,
        string_map_to_domain(dto.metadata)?,
    )?)
}

fn ingest_coordinate_to_domain(
    dto: IngestTemporalCoordinateDto,
) -> Result<IngestTemporalCoordinate, MappingError> {
    Ok(IngestTemporalCoordinate::new(
        DimensionRef::parse(dto.dimension)?,
        NonEmptyString::parse(
            dto.scope_id,
            "ingest.memory.entries[].coordinates[].scope_id",
        )?,
        optional_timestamp(
            dto.occurred_at,
            "ingest.memory.entries[].coordinates[].occurred_at",
        )?,
        optional_timestamp(
            dto.observed_at,
            "ingest.memory.entries[].coordinates[].observed_at",
        )?,
        optional_timestamp(
            dto.ingested_at,
            "ingest.memory.entries[].coordinates[].ingested_at",
        )?,
        optional_timestamp(
            dto.valid_from,
            "ingest.memory.entries[].coordinates[].valid_from",
        )?,
        optional_timestamp(
            dto.valid_until,
            "ingest.memory.entries[].coordinates[].valid_until",
        )?,
        dto.sequence
            .map(|value| {
                PositiveCount::parse(value, "ingest.memory.entries[].coordinates[].sequence")
            })
            .transpose()?,
        dto.rank
            .map(|value| PositiveCount::parse(value, "ingest.memory.entries[].coordinates[].rank"))
            .transpose()?,
        string_map_to_domain(dto.metadata)?,
    )?)
}

fn ingest_relation_to_domain(dto: IngestRelationDto) -> Result<IngestRelation, MappingError> {
    Ok(IngestRelation::new(
        MemoryRef::parse(dto.source_ref)?,
        MemoryRef::parse(dto.target_ref)?,
        IngestRelationType::parse(dto.rel)?,
        IngestRelationClass::parse(&dto.semantic_class)?,
        dto.why
            .map(|why| NonEmptyString::parse(why, "ingest.memory.relations[].why"))
            .transpose()?,
        dto.evidence
            .map(|evidence| NonEmptyString::parse(evidence, "ingest.memory.relations[].evidence"))
            .transpose()?,
        dto.confidence
            .map(|confidence| IngestConfidence::parse(&confidence))
            .transpose()?,
        dto.sequence
            .map(|value| PositiveCount::parse(value, "ingest.memory.relations[].sequence"))
            .transpose()?,
    )?)
}

fn ingest_evidence_to_domain(dto: IngestEvidenceDto) -> Result<IngestEvidence, MappingError> {
    let mut supports = Vec::with_capacity(dto.supports.len());
    for support in dto.supports {
        supports.push(MemoryRef::parse(support)?);
    }
    Ok(IngestEvidence::new(
        NonEmptyString::parse(dto.id, "ingest.memory.evidence[].id")?,
        supports,
        NonEmptyString::parse(dto.text, "ingest.memory.evidence[].text")?,
        dto.source
            .map(|source| NonEmptyString::parse(source, "ingest.memory.evidence[].source"))
            .transpose()?,
        optional_timestamp(dto.time, "ingest.memory.evidence[].time")?,
        string_map_to_domain(dto.metadata)?,
    ))
}

fn ingest_provenance_to_domain(dto: IngestProvenanceDto) -> Result<IngestProvenance, MappingError> {
    Ok(IngestProvenance::new(
        IngestSourceKind::parse(&dto.source_kind)?,
        NonEmptyString::parse(dto.source_agent, "ingest.provenance.source_agent")?,
        required_timestamp(&dto.observed_at, "ingest.provenance.observed_at")?,
        dto.correlation_id
            .map(|id| NonEmptyString::parse(id, "ingest.provenance.correlation_id"))
            .transpose()?,
        dto.causation_id
            .map(|id| NonEmptyString::parse(id, "ingest.provenance.causation_id"))
            .transpose()?,
    ))
}

fn ingest_memory_to_dto(domain: &IngestMemory) -> IngestMemoryDto {
    IngestMemoryDto {
        dimensions: domain
            .dimensions()
            .iter()
            .map(ingest_dimension_to_dto)
            .collect(),
        entries: domain.entries().iter().map(ingest_entry_to_dto).collect(),
        relations: domain
            .relations()
            .iter()
            .map(ingest_relation_to_dto)
            .collect(),
        evidence: domain
            .evidence()
            .iter()
            .map(ingest_evidence_to_dto)
            .collect(),
    }
}

fn ingest_dimension_to_dto(domain: &IngestDimension) -> IngestDimensionDto {
    IngestDimensionDto {
        id: domain.id().as_str().to_string(),
        kind: domain.kind().as_str().to_string(),
        title: domain.title().map(|title| title.as_str().to_string()),
        metadata: string_map_to_dto(domain.metadata()),
    }
}

fn ingest_entry_to_dto(domain: &IngestEntry) -> IngestEntryDto {
    IngestEntryDto {
        id: domain.id().as_str().to_string(),
        kind: domain.kind().as_str().to_string(),
        text: domain.text().as_str().to_string(),
        coordinates: domain
            .coordinates()
            .iter()
            .map(ingest_coordinate_to_dto)
            .collect(),
        metadata: string_map_to_dto(domain.metadata()),
    }
}

fn ingest_coordinate_to_dto(domain: &IngestTemporalCoordinate) -> IngestTemporalCoordinateDto {
    IngestTemporalCoordinateDto {
        dimension: domain.dimension().as_str().to_string(),
        scope_id: domain.scope_id().as_str().to_string(),
        occurred_at: optional_timestamp_to_dto(domain.occurred_at()),
        observed_at: optional_timestamp_to_dto(domain.observed_at()),
        ingested_at: optional_timestamp_to_dto(domain.ingested_at()),
        valid_from: optional_timestamp_to_dto(domain.valid_from()),
        valid_until: optional_timestamp_to_dto(domain.valid_until()),
        sequence: domain.sequence().map(PositiveCount::as_usize),
        rank: domain.rank().map(PositiveCount::as_usize),
        metadata: string_map_to_dto(domain.metadata()),
    }
}

fn ingest_relation_to_dto(domain: &IngestRelation) -> IngestRelationDto {
    IngestRelationDto {
        source_ref: domain.source_ref().as_str().to_string(),
        target_ref: domain.target_ref().as_str().to_string(),
        rel: domain.relation_type().as_str().to_string(),
        semantic_class: domain.semantic_class().as_str().to_string(),
        why: domain.why().map(|why| why.as_str().to_string()),
        evidence: domain
            .evidence()
            .map(|evidence| evidence.as_str().to_string()),
        confidence: domain
            .confidence()
            .map(|confidence| confidence.as_str().to_string()),
        sequence: domain.sequence().map(PositiveCount::as_usize),
    }
}

fn ingest_evidence_to_dto(domain: &IngestEvidence) -> IngestEvidenceDto {
    IngestEvidenceDto {
        id: domain.id().as_str().to_string(),
        supports: domain
            .supports()
            .iter()
            .map(|support| support.as_str().to_string())
            .collect(),
        text: domain.text().as_str().to_string(),
        source: domain.source().map(|source| source.as_str().to_string()),
        time: optional_timestamp_to_dto(domain.time()),
        metadata: string_map_to_dto(domain.metadata()),
    }
}

fn ingest_provenance_to_dto(domain: &IngestProvenance) -> IngestProvenanceDto {
    IngestProvenanceDto {
        source_kind: domain.source_kind().as_str().to_string(),
        source_agent: domain.source_agent().as_str().to_string(),
        observed_at: timestamp_to_dto(domain.observed_at()),
        correlation_id: domain.correlation_id().map(|id| id.as_str().to_string()),
        causation_id: domain.causation_id().map(|id| id.as_str().to_string()),
    }
}

fn string_map_to_domain(raw: BTreeMap<String, String>) -> Result<StringMap, MappingError> {
    let mut entries = BTreeMap::new();
    for (key, value) in raw {
        entries.insert(
            NonEmptyString::parse(key, "metadata.key")?,
            NonEmptyString::parse(value, "metadata.value")?,
        );
    }
    Ok(StringMap::new(entries))
}

fn string_map_to_dto(domain: &StringMap) -> BTreeMap<String, String> {
    domain
        .entries()
        .iter()
        .map(|(key, value)| (key.as_str().to_string(), value.as_str().to_string()))
        .collect()
}

fn optional_timestamp(
    value: Option<String>,
    field: &'static str,
) -> Result<Option<SystemTime>, MappingError> {
    value.map(|raw| required_timestamp(&raw, field)).transpose()
}

fn required_timestamp(value: &str, field: &'static str) -> Result<SystemTime, MappingError> {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|date_time| SystemTime::from(date_time.with_timezone(&chrono::Utc)))
        .map_err(|err| MappingError::InvalidToolArguments {
            tool: "kernel_ingest",
            message: format!("{field} must be RFC3339: {err}"),
        })
}

fn optional_timestamp_to_dto(value: Option<SystemTime>) -> Option<String> {
    value.map(timestamp_to_dto)
}

fn timestamp_to_dto(value: SystemTime) -> String {
    let date_time = chrono::DateTime::<chrono::Utc>::from(value);
    date_time.to_rfc3339()
}
