use super::super::{SemanticDagNodeId, SemanticDagPayload, SemanticDagSnapshot, SemanticDagType};
use super::model::{SealedSemanticDagError, SealedSemanticDagFailure, SealedSemanticDagOwner};
use super::validated_types::{
    dag_type, field_type, return_type, runtime_type, structural_type, ExpectedType,
};
use super::SealedSemanticDagRuntime;
use crate::{
    InlineStructuralValue, StructuralFieldMetadata, StructuralKind, StructuralLayoutKind,
    StructuralType, StructuralTypeMetadata, ValidatedChunk,
};

impl SealedSemanticDagRuntime {
    pub fn rehydrate_validated_return(
        &mut self,
        chunk: &ValidatedChunk,
        snapshot: SemanticDagSnapshot,
    ) -> Result<SealedSemanticDagOwner, SealedSemanticDagFailure> {
        let (root, closure) = match validated_shape(chunk, &snapshot) {
            Ok(plan) => plan,
            Err(error) => return Err(SealedSemanticDagFailure::new(error, snapshot)),
        };
        self.rehydrate(snapshot, root, &closure)
    }
}

fn validated_shape(
    chunk: &ValidatedChunk,
    snapshot: &SemanticDagSnapshot,
) -> Result<(SemanticDagType, Vec<SemanticDagType>), SealedSemanticDagError> {
    let root = return_type(chunk)?;
    let root_type = dag_type(runtime_type(chunk, root)?)?;
    let count = snapshot.nodes().len();
    let mut expected = Vec::new();
    expected
        .try_reserve_exact(count)
        .map_err(|_| SealedSemanticDagError::AllocationFailed)?;
    expected.resize(count, None);
    let capacity = usize::try_from(snapshot.metrics().fields)
        .ok()
        .and_then(|edges| edges.checked_add(1))
        .ok_or(SealedSemanticDagError::ArithmeticOverflow)?;
    let mut pending = Vec::new();
    pending
        .try_reserve_exact(capacity)
        .map_err(|_| SealedSemanticDagError::AllocationFailed)?;
    pending.push((snapshot.root(), root));
    while let Some((id, value_type)) = pending.pop() {
        let index = id.get() as usize;
        let slot = expected
            .get_mut(index)
            .ok_or(SealedSemanticDagError::ValidatedShapeMismatch)?;
        if let Some(prior) = *slot {
            if prior != value_type {
                return Err(SealedSemanticDagError::ValidatedShapeMismatch);
            }
            continue;
        }
        let node = snapshot
            .nodes()
            .get(index)
            .ok_or(SealedSemanticDagError::ValidatedShapeMismatch)?;
        let runtime_type = runtime_type(chunk, value_type)?;
        if node.value_type != dag_type(runtime_type)? {
            return Err(SealedSemanticDagError::ValidatedShapeMismatch);
        }
        *slot = Some(value_type);
        validate_node(chunk, node, value_type, runtime_type, &mut pending)?;
    }
    let mut closure = Vec::new();
    closure
        .try_reserve_exact(count)
        .map_err(|_| SealedSemanticDagError::AllocationFailed)?;
    for value_type in expected {
        let value_type = value_type.ok_or(SealedSemanticDagError::ValidatedShapeMismatch)?;
        closure.push(dag_type(runtime_type(chunk, value_type)?)?);
    }
    closure.sort_unstable();
    closure.dedup();
    Ok((root_type, closure))
}

fn validate_node(
    chunk: &ValidatedChunk,
    node: &super::super::SemanticDagNode,
    authority: ExpectedType,
    expected: StructuralType,
    pending: &mut Vec<(SemanticDagNodeId, ExpectedType)>,
) -> Result<(), SealedSemanticDagError> {
    match (expected.kind, &node.payload) {
        (StructuralKind::Unit, SemanticDagPayload::Inline(InlineStructuralValue::Unit))
        | (StructuralKind::Bool, SemanticDagPayload::Inline(InlineStructuralValue::Bool(_)))
        | (StructuralKind::I64, SemanticDagPayload::Inline(InlineStructuralValue::I64(_)))
        | (StructuralKind::F64, SemanticDagPayload::Inline(InlineStructuralValue::F64Bits(_)))
        | (StructuralKind::Static, SemanticDagPayload::Static(_))
        | (StructuralKind::String, SemanticDagPayload::String(_))
        | (StructuralKind::Path, SemanticDagPayload::Path(_)) => Ok(()),
        (StructuralKind::Product, SemanticDagPayload::Product(children)) => {
            let ExpectedType::Structural(id) = authority else {
                return Err(SealedSemanticDagError::ValidatedShapeMismatch);
            };
            let metadata = structural_type(chunk, id)?;
            let StructuralLayoutKind::Product { fields, .. } = layout(chunk, metadata)? else {
                return Err(SealedSemanticDagError::ValidatedShapeMismatch);
            };
            push_fields(chunk, children, fields, pending)
        }
        _ => Err(SealedSemanticDagError::ValidatedShapeMismatch),
    }
}

fn push_fields(
    chunk: &ValidatedChunk,
    children: &[SemanticDagNodeId],
    fields: &[StructuralFieldMetadata],
    pending: &mut Vec<(SemanticDagNodeId, ExpectedType)>,
) -> Result<(), SealedSemanticDagError> {
    if children.len() != fields.len() {
        return Err(SealedSemanticDagError::ValidatedShapeMismatch);
    }
    for (&child, field) in children.iter().zip(fields).rev() {
        pending.push((child, field_type(chunk, field)?));
    }
    Ok(())
}

fn layout<'a>(
    chunk: &'a ValidatedChunk,
    metadata: &StructuralTypeMetadata,
) -> Result<&'a StructuralLayoutKind, SealedSemanticDagError> {
    chunk
        .structural_layouts()
        .get(metadata.layout.index())
        .filter(|layout| layout.id == metadata.layout)
        .map(|layout| &layout.kind)
        .ok_or(SealedSemanticDagError::ValidatedShapeMismatch)
}
