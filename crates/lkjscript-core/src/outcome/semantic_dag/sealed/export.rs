use super::super::{
    SemanticDagKind, SemanticDagNode, SemanticDagNodeId, SemanticDagPayload, SemanticDagSnapshot,
};
use super::cells::{SealedDagCell, SealedDagNodeCell, SealedDagNodePayload};
use super::model::{SealedSemanticDagBorrow, SealedSemanticDagError};
use super::{SealedSemanticDagRuntime, TypedSealedDagStore, SEALED_DAG_BYTE_CHUNK};

impl SealedSemanticDagRuntime {
    pub fn export_snapshot(
        &self,
        borrow: &SealedSemanticDagBorrow,
    ) -> Result<SemanticDagSnapshot, SealedSemanticDagError> {
        if borrow.nodes == 0 || borrow.root != borrow.nodes - 1 || borrow.cells < borrow.nodes {
            return Err(SealedSemanticDagError::CorruptRegion);
        }
        let typed = self.store(borrow.store, borrow.value_type)?;
        let mut nodes = Vec::new();
        nodes
            .try_reserve_exact(borrow.nodes as usize)
            .map_err(|_| SealedSemanticDagError::AllocationFailed)?;
        let mut next_auxiliary = borrow.nodes;
        for slot in 0..borrow.nodes {
            let SealedDagCell::Node(node) = cell(typed, borrow, slot)? else {
                return Err(SealedSemanticDagError::CorruptRegion);
            };
            next_auxiliary = canonical_auxiliary(node.payload, next_auxiliary)?;
            let payload = export_payload(typed, borrow, node)?;
            nodes.push(SemanticDagNode::new(node.value_type, payload));
        }
        if next_auxiliary != borrow.cells
            || nodes
                .get(borrow.root as usize)
                .is_none_or(|node| node.value_type != borrow.value_type)
        {
            return Err(SealedSemanticDagError::CorruptRegion);
        }
        SemanticDagSnapshot::new(nodes, SemanticDagNodeId::new(borrow.root))
            .map_err(|_| SealedSemanticDagError::CorruptRegion)
    }
}

fn export_payload(
    typed: &TypedSealedDagStore,
    borrow: &SealedSemanticDagBorrow,
    node: SealedDagNodeCell,
) -> Result<SemanticDagPayload, SealedSemanticDagError> {
    match node.payload {
        SealedDagNodePayload::Inline(value) => Ok(SemanticDagPayload::Inline(value)),
        SealedDagNodePayload::Static(value) => Ok(SemanticDagPayload::Static(value)),
        SealedDagNodePayload::Bytes {
            first,
            chunks,
            length,
        } => export_bytes(typed, borrow, node.value_type.kind, first, chunks, length),
        SealedDagNodePayload::Product { first, fields } => Ok(SemanticDagPayload::Product(
            export_children(typed, borrow, first, fields)?,
        )),
        SealedDagNodePayload::Enum { tag, first, fields } => Ok(SemanticDagPayload::Enum {
            tag,
            fields: export_children(typed, borrow, first, fields)?,
        }),
        SealedDagNodePayload::EmptyList => Ok(SemanticDagPayload::EmptyList),
        SealedDagNodePayload::List { head, tail } => Ok(SemanticDagPayload::List { head, tail }),
    }
}

fn export_children(
    typed: &TypedSealedDagStore,
    borrow: &SealedSemanticDagBorrow,
    first: u32,
    length: u32,
) -> Result<Vec<SemanticDagNodeId>, SealedSemanticDagError> {
    require_range(borrow, first, length)?;
    let mut children = Vec::new();
    children
        .try_reserve_exact(length as usize)
        .map_err(|_| SealedSemanticDagError::AllocationFailed)?;
    for slot in first..first + length {
        let SealedDagCell::Child(child) = cell(typed, borrow, slot)? else {
            return Err(SealedSemanticDagError::CorruptRegion);
        };
        children.push(child);
    }
    Ok(children)
}

fn export_bytes(
    typed: &TypedSealedDagStore,
    borrow: &SealedSemanticDagBorrow,
    kind: SemanticDagKind,
    first: u32,
    chunks: u32,
    length: u32,
) -> Result<SemanticDagPayload, SealedSemanticDagError> {
    require_range(borrow, first, chunks)?;
    let expected = (length as usize).div_ceil(SEALED_DAG_BYTE_CHUNK);
    if usize::try_from(chunks).ok() != Some(expected) {
        return Err(SealedSemanticDagError::CorruptRegion);
    }
    let mut bytes = Vec::new();
    bytes
        .try_reserve_exact(length as usize)
        .map_err(|_| SealedSemanticDagError::AllocationFailed)?;
    for (index, slot) in (first..first + chunks).enumerate() {
        let SealedDagCell::Bytes {
            length: stored_length,
            bytes: data,
        } = cell(typed, borrow, slot)?
        else {
            return Err(SealedSemanticDagError::CorruptRegion);
        };
        let consumed = index
            .checked_mul(SEALED_DAG_BYTE_CHUNK)
            .ok_or(SealedSemanticDagError::ArithmeticOverflow)?;
        let expected_length = (length as usize - consumed).min(SEALED_DAG_BYTE_CHUNK);
        if usize::from(stored_length) != expected_length
            || data[expected_length..].iter().any(|byte| *byte != 0)
        {
            return Err(SealedSemanticDagError::CorruptRegion);
        }
        bytes.extend_from_slice(&data[..expected_length]);
    }
    if bytes.len() != length as usize {
        return Err(SealedSemanticDagError::CorruptRegion);
    }
    match kind {
        SemanticDagKind::String => Ok(SemanticDagPayload::String(bytes)),
        SemanticDagKind::Path => Ok(SemanticDagPayload::Path(bytes)),
        SemanticDagKind::Bytes => Ok(SemanticDagPayload::Bytes(bytes)),
        _ => Err(SealedSemanticDagError::CorruptRegion),
    }
}

fn canonical_auxiliary(
    payload: SealedDagNodePayload,
    expected_first: u32,
) -> Result<u32, SealedSemanticDagError> {
    let range = match payload {
        SealedDagNodePayload::Bytes { first, chunks, .. } => Some((first, chunks)),
        SealedDagNodePayload::Product { first, fields }
        | SealedDagNodePayload::Enum { first, fields, .. } => Some((first, fields)),
        _ => None,
    };
    let Some((first, length)) = range else {
        return Ok(expected_first);
    };
    if first != expected_first {
        return Err(SealedSemanticDagError::CorruptRegion);
    }
    first
        .checked_add(length)
        .ok_or(SealedSemanticDagError::ArithmeticOverflow)
}

fn cell(
    typed: &TypedSealedDagStore,
    borrow: &SealedSemanticDagBorrow,
    slot: u32,
) -> Result<SealedDagCell, SealedSemanticDagError> {
    if slot >= borrow.cells {
        return Err(SealedSemanticDagError::CorruptRegion);
    }
    Ok(*typed.store.borrowed_at(&borrow.borrow, slot)?)
}

fn require_range(
    borrow: &SealedSemanticDagBorrow,
    first: u32,
    length: u32,
) -> Result<(), SealedSemanticDagError> {
    if first < borrow.nodes
        || first
            .checked_add(length)
            .is_none_or(|end| end > borrow.cells)
    {
        Err(SealedSemanticDagError::CorruptRegion)
    } else {
        Ok(())
    }
}
