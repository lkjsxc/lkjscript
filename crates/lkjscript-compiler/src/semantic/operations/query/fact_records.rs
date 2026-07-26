use crate::semantic::schema::{
    FactCertainty, FactRecord, FactReference, FactSchema, FactValue, ProducerRecord, ProducerStage,
    RelationCardinality, UnavailableReason,
};

fn producer(stage: ProducerStage) -> ProducerRecord {
    ProducerRecord {
        component: "lkjscript-compiler".to_string(),
        stage,
        build: env!("CARGO_PKG_VERSION").to_string(),
    }
}

pub(super) fn available_value(
    schema: FactSchema,
    stage: ProducerStage,
    revision: &str,
    value: FactValue,
) -> FactRecord {
    available(schema, stage, revision, vec![value], Vec::new())
}

pub(super) fn available_reference(
    schema: FactSchema,
    stage: ProducerStage,
    revision: &str,
    reference: FactReference,
) -> FactRecord {
    available(schema, stage, revision, Vec::new(), vec![reference])
}

fn available(
    schema: FactSchema,
    stage: ProducerStage,
    revision: &str,
    values: Vec<FactValue>,
    references: Vec<FactReference>,
) -> FactRecord {
    FactRecord::Available {
        producer: producer(stage),
        fact_schema: schema,
        fact_contract: crate::semantic::CONTRACT.to_hex(),
        source_revision: revision.to_string(),
        derived_artifact_identity: format!("{}:{schema:?}:{revision}", crate::semantic::CONTRACT),
        certainty: FactCertainty::Guaranteed,
        cardinality: RelationCardinality::One,
        values,
        references,
    }
}

pub(super) fn unavailable(
    schema: FactSchema,
    stage: ProducerStage,
    revision: &str,
    reason: UnavailableReason,
) -> FactRecord {
    FactRecord::Unavailable {
        producer: producer(stage),
        fact_schema: schema,
        fact_contract: crate::semantic::CONTRACT.to_hex(),
        source_revision: revision.to_string(),
        derived_artifact_identity: None,
        certainty: FactCertainty::Informational,
        cardinality: RelationCardinality::Zero,
        reason,
    }
}
