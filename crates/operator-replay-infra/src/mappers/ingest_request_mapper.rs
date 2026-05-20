use std::time::SystemTime;

use operator_shared_domain::tool_arguments::ingest_arguments::IngestArguments;
use operator_shared_domain::tool_arguments::ingest_dimension::IngestDimension;
use operator_shared_domain::tool_arguments::ingest_entry::IngestEntry;
use operator_shared_domain::tool_arguments::ingest_evidence::IngestEvidence;
use operator_shared_domain::tool_arguments::ingest_provenance::IngestProvenance;
use operator_shared_domain::tool_arguments::ingest_relation::IngestRelation;
use operator_shared_domain::tool_arguments::ingest_temporal_coordinate::IngestTemporalCoordinate;
use operator_shared_domain::value_objects::memory_ref::MemoryRef;
use operator_shared_domain::value_objects::string_map::StringMap;
use serde_json::{Map, Value, json};

#[derive(Debug)]
pub struct IngestRequestMapper;

impl IngestRequestMapper {
    pub fn to_json(args: &IngestArguments) -> Value {
        let mut root = Map::new();
        root.insert("about".to_string(), json!(args.about().as_str()));
        root.insert("memory".to_string(), ingest_memory_json(args));
        if let Some(provenance) = args.provenance() {
            root.insert("provenance".to_string(), ingest_provenance_json(provenance));
        }
        root.insert(
            "idempotency_key".to_string(),
            json!(args.idempotency_key().as_str()),
        );
        root.insert("dry_run".to_string(), json!(args.dry_run()));
        Value::Object(root)
    }
}

fn ingest_memory_json(args: &IngestArguments) -> Value {
    json!({
        "dimensions": args.memory().dimensions().iter().map(ingest_dimension_json).collect::<Vec<_>>(),
        "entries": args.memory().entries().iter().map(ingest_entry_json).collect::<Vec<_>>(),
        "relations": args.memory().relations().iter().map(ingest_relation_json).collect::<Vec<_>>(),
        "evidence": args.memory().evidence().iter().map(ingest_evidence_json).collect::<Vec<_>>(),
    })
}

fn ingest_dimension_json(dimension: &IngestDimension) -> Value {
    let mut body = Map::new();
    body.insert("id".to_string(), json!(dimension.id().as_str()));
    body.insert("kind".to_string(), json!(dimension.kind().as_str()));
    if let Some(title) = dimension.title() {
        body.insert("title".to_string(), json!(title.as_str()));
    }
    body.insert(
        "metadata".to_string(),
        string_map_json(dimension.metadata()),
    );
    Value::Object(body)
}

fn ingest_entry_json(entry: &IngestEntry) -> Value {
    json!({
        "id": entry.id().as_str(),
        "kind": entry.kind().as_str(),
        "text": entry.text().as_str(),
        "coordinates": entry.coordinates().iter().map(ingest_coordinate_json).collect::<Vec<_>>(),
        "metadata": string_map_json(entry.metadata()),
    })
}

fn ingest_coordinate_json(coordinate: &IngestTemporalCoordinate) -> Value {
    let mut body = Map::new();
    body.insert(
        "dimension".to_string(),
        json!(coordinate.dimension().as_str()),
    );
    body.insert(
        "scope_id".to_string(),
        json!(coordinate.scope_id().as_str()),
    );
    insert_time(&mut body, "occurred_at", coordinate.occurred_at());
    insert_time(&mut body, "observed_at", coordinate.observed_at());
    insert_time(&mut body, "ingested_at", coordinate.ingested_at());
    insert_time(&mut body, "valid_from", coordinate.valid_from());
    insert_time(&mut body, "valid_until", coordinate.valid_until());
    if let Some(sequence) = coordinate.sequence() {
        body.insert("sequence".to_string(), json!(sequence.as_usize()));
    }
    if let Some(rank) = coordinate.rank() {
        body.insert("rank".to_string(), json!(rank.as_usize()));
    }
    body.insert(
        "metadata".to_string(),
        string_map_json(coordinate.metadata()),
    );
    Value::Object(body)
}

fn ingest_relation_json(relation: &IngestRelation) -> Value {
    let mut body = Map::new();
    body.insert("from".to_string(), json!(relation.source_ref().as_str()));
    body.insert("to".to_string(), json!(relation.target_ref().as_str()));
    body.insert("rel".to_string(), json!(relation.relation_type().as_str()));
    body.insert(
        "class".to_string(),
        json!(relation.semantic_class().as_str()),
    );
    if let Some(why) = relation.why() {
        body.insert("why".to_string(), json!(why.as_str()));
    }
    if let Some(evidence) = relation.evidence() {
        body.insert("evidence".to_string(), json!(evidence.as_str()));
    }
    if let Some(confidence) = relation.confidence() {
        body.insert("confidence".to_string(), json!(confidence.as_str()));
    }
    if let Some(sequence) = relation.sequence() {
        body.insert("sequence".to_string(), json!(sequence.as_usize()));
    }
    Value::Object(body)
}

fn ingest_evidence_json(evidence: &IngestEvidence) -> Value {
    let mut body = Map::new();
    body.insert("id".to_string(), json!(evidence.id().as_str()));
    body.insert(
        "supports".to_string(),
        json!(
            evidence
                .supports()
                .iter()
                .map(MemoryRef::as_str)
                .collect::<Vec<_>>()
        ),
    );
    body.insert("text".to_string(), json!(evidence.text().as_str()));
    if let Some(source) = evidence.source() {
        body.insert("source".to_string(), json!(source.as_str()));
    }
    insert_time(&mut body, "time", evidence.time());
    body.insert("metadata".to_string(), string_map_json(evidence.metadata()));
    Value::Object(body)
}

fn ingest_provenance_json(provenance: &IngestProvenance) -> Value {
    let mut body = Map::new();
    body.insert(
        "source_kind".to_string(),
        json!(provenance.source_kind().as_str()),
    );
    body.insert(
        "source_agent".to_string(),
        json!(provenance.source_agent().as_str()),
    );
    body.insert(
        "observed_at".to_string(),
        json!(timestamp_string(provenance.observed_at())),
    );
    if let Some(correlation_id) = provenance.correlation_id() {
        body.insert("correlation_id".to_string(), json!(correlation_id.as_str()));
    }
    if let Some(causation_id) = provenance.causation_id() {
        body.insert("causation_id".to_string(), json!(causation_id.as_str()));
    }
    Value::Object(body)
}

fn insert_time(body: &mut Map<String, Value>, key: &str, value: Option<SystemTime>) {
    if let Some(value) = value {
        body.insert(key.to_string(), json!(timestamp_string(value)));
    }
}

fn timestamp_string(value: SystemTime) -> String {
    let date_time = chrono::DateTime::<chrono::Utc>::from(value);
    date_time.to_rfc3339()
}

fn string_map_json(map: &StringMap) -> Value {
    Value::Object(
        map.entries()
            .iter()
            .map(|(key, value)| {
                (
                    key.as_str().to_string(),
                    Value::String(value.as_str().to_string()),
                )
            })
            .collect(),
    )
}
