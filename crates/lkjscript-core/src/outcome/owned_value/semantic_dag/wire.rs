use crate::{
    LayoutIdentity, SemanticDagKind, SemanticDagPayload, SemanticDagType, SemanticTypeIdentity,
};

use super::semantic_dag::{
    dag_child_index, dag_children, validate_dag_kind, validate_dag_path,
};

fn encode_semantic_dag(
    out: &mut Encoder,
    snapshot: &SemanticDagSnapshot,
) -> Result<()> {
    snapshot.validate_encode()?;
    out.u64(
        u64::try_from(snapshot.nodes().len())
            .map_err(|_| Error::msg("semantic DAG node count exceeds u64"))?,
    )?;
    out.u64(snapshot.root().get())?;
    for node in snapshot.nodes() {
        encode_semantic_dag_type(out, node.value_type)?;
        encode_semantic_dag_payload(out, &node.payload)?;
    }
    Ok(())
}

fn encode_semantic_dag_type(out: &mut Encoder, value: SemanticDagType) -> Result<()> {
    out.u64(value.layout.get())?;
    out.u64(value.semantic_type.get())?;
    out.u8(semantic_dag_kind_tag(value.kind))
}

fn encode_semantic_dag_payload(out: &mut Encoder, value: &SemanticDagPayload) -> Result<()> {
    match value {
        SemanticDagPayload::Inline(InlineStructuralValue::Unit) => Ok(()),
        SemanticDagPayload::Inline(InlineStructuralValue::Bool(value)) => out.u8(u8::from(*value)),
        SemanticDagPayload::Inline(InlineStructuralValue::I64(value)) => out.u64(*value as u64),
        SemanticDagPayload::Inline(InlineStructuralValue::F64Bits(value)) => out.u64(*value),
        SemanticDagPayload::Static(value) => encode_structural_static(out, *value),
        SemanticDagPayload::String(bytes)
        | SemanticDagPayload::Path(bytes)
        | SemanticDagPayload::Bytes(bytes) => out.bytes(bytes),
        SemanticDagPayload::Product(fields) => encode_semantic_dag_fields(out, fields),
        SemanticDagPayload::Enum { tag, fields } => {
            out.u64(*tag)?;
            encode_semantic_dag_fields(out, fields)
        }
        SemanticDagPayload::EmptyList => Ok(()),
        SemanticDagPayload::List { head, tail } => {
            out.u64(head.get())?;
            out.u64(tail.get())
        }
    }
}

fn encode_semantic_dag_fields(
    out: &mut Encoder,
    fields: &[SemanticDagNodeId],
) -> Result<()> {
    out.u64(
        u64::try_from(fields.len())
            .map_err(|_| Error::msg("semantic DAG edge count exceeds u64"))?,
    )?;
    for field in fields {
        out.u64(field.get())?;
    }
    Ok(())
}
