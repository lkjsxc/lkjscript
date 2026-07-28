mod binding;
mod fact_records;

use binding::binding_fact;
use fact_records::{available_value, unavailable};

use crate::semantic::codec::error;
use crate::semantic::schema::{
    FactSchema, FactValue, NodeFacts, NodeQueryRecord, ProducerStage, ProtocolError,
    ProtocolErrorCode, SemanticEffect, UnavailableReason,
};
use crate::source::{NodeKind, ValidatedSourceTree};

pub(crate) fn query(
    tree: &ValidatedSourceTree,
    index: u32,
) -> Result<NodeQueryRecord, ProtocolError> {
    let node = tree.nodes().get(index as usize).ok_or_else(|| {
        error(
            ProtocolErrorCode::UnknownNode,
            format!("unknown revision-scoped node index {index}"),
        )
    })?;
    let revision = tree.revision().to_hex();
    let literal_type = match node.kind() {
        NodeKind::I64Literal => Some("i64"),
        NodeKind::F64Literal => Some("f64"),
        NodeKind::BoolLiteral => Some("bool"),
        NodeKind::UnitLiteral => Some("unit"),
        NodeKind::StringLiteral => Some("string"),
        NodeKind::BytesLiteral => Some("bytes"),
        NodeKind::Symbol | NodeKind::Call => None,
    };
    let static_type = literal_type.map_or_else(
        || {
            unavailable(
                FactSchema::StaticType,
                ProducerStage::Hir,
                &revision,
                UnavailableReason::NoExactSourceCorrelation,
            )
        },
        |canonical| {
            available_value(
                FactSchema::StaticType,
                ProducerStage::SourceResolution,
                &revision,
                FactValue::StaticType {
                    canonical: canonical.to_string(),
                },
            )
        },
    );
    let semantic_effects = literal_type.map_or_else(
        || {
            unavailable(
                FactSchema::SemanticEffects,
                ProducerStage::Hir,
                &revision,
                UnavailableReason::NoExactSourceCorrelation,
            )
        },
        |_| {
            available_value(
                FactSchema::SemanticEffects,
                ProducerStage::SourceResolution,
                &revision,
                FactValue::SemanticEffects {
                    effects: Vec::<SemanticEffect>::new(),
                },
            )
        },
    );
    Ok(NodeQueryRecord {
        node: crate::semantic::tree::node_record(tree, node),
        facts: NodeFacts {
            binding: binding_fact(tree, index, node, &revision),
            hir: unavailable_fact(FactSchema::Hir, ProducerStage::Hir, &revision),
            static_type,
            semantic_effects,
            ownership_place_loan: unavailable_fact(
                FactSchema::OwnershipPlaceLoan,
                ProducerStage::Ownership,
                &revision,
            ),
            control_flow_graph: unavailable_fact(
                FactSchema::ControlFlowGraph,
                ProducerStage::Ssa,
                &revision,
            ),
            ssa_values_blocks: unavailable_fact(
                FactSchema::SsaValuesBlocks,
                ProducerStage::Ssa,
                &revision,
            ),
            frame_states_safepoints_roots: unavailable(
                FactSchema::FrameStatesSafepointsRoots,
                ProducerStage::Runtime,
                &revision,
                UnavailableReason::NotProduced,
            ),
            layout: unavailable_fact(FactSchema::Layout, ProducerStage::Runtime, &revision),
            proof: unavailable_fact(FactSchema::Proof, ProducerStage::ProofChecker, &revision),
            bytecode: unavailable_fact(FactSchema::Bytecode, ProducerStage::Bytecode, &revision),
            native_location: unavailable_fact(
                FactSchema::NativeLocation,
                ProducerStage::Native,
                &revision,
            ),
        },
    })
}

fn unavailable_fact(
    schema: FactSchema,
    stage: ProducerStage,
    revision: &str,
) -> crate::semantic::schema::FactRecord {
    unavailable(
        schema,
        stage,
        revision,
        UnavailableReason::NoExactSourceCorrelation,
    )
}
