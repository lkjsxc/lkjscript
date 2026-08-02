use super::super::{SemanticDagPayload, SemanticDagSnapshot, SemanticDagType};
use super::cells::{SealedDagCell, SealedDagNodeCell, SealedDagNodePayload};
use super::model::SealedSemanticDagError;
use super::{SealedSemanticDagRuntime, SEALED_DAG_BYTE_CHUNK};
use crate::{StructuralLimit, MAX_MEMORY_WITNESSES};

pub(super) struct RehydrationPlan {
    pub cells: Vec<SealedDagCell>,
    pub cell_count: u32,
    pub nodes: u32,
    pub root: u32,
}

impl SealedSemanticDagRuntime {
    pub(super) fn plan(
        &self,
        snapshot: &SemanticDagSnapshot,
        expected: SemanticDagType,
        type_closure: &[SemanticDagType],
    ) -> Result<RehydrationPlan, SealedSemanticDagError> {
        validate_type_closure(snapshot, expected, type_closure)?;
        let node_count = u32::try_from(snapshot.nodes().len())
            .map_err(|_| SealedSemanticDagError::ArithmeticOverflow)?;
        let mut total = node_count;
        for node in snapshot.nodes() {
            total = total
                .checked_add(auxiliary_cells(&node.payload)?)
                .ok_or(SealedSemanticDagError::ArithmeticOverflow)?;
        }
        if total > self.limits.max_objects_per_domain {
            return Err(crate::StructuralError::LimitExceeded(StructuralLimit::Objects).into());
        }
        let bytes = u64::from(total)
            .checked_mul(std::mem::size_of::<SealedDagCell>() as u64)
            .ok_or(SealedSemanticDagError::ArithmeticOverflow)?;
        if bytes > self.limits.max_bytes_per_domain {
            return Err(crate::StructuralError::LimitExceeded(StructuralLimit::Bytes).into());
        }
        if snapshot.metrics().fields > self.limits.max_dependencies {
            return Err(
                crate::StructuralError::LimitExceeded(StructuralLimit::Dependencies).into(),
            );
        }
        let mut cells = Vec::new();
        cells
            .try_reserve_exact(total as usize)
            .map_err(|_| SealedSemanticDagError::AllocationFailed)?;
        let mut next = node_count;
        for node in snapshot.nodes() {
            let (payload, used) = planned_payload(&node.payload, next)?;
            cells.push(SealedDagCell::Node(SealedDagNodeCell {
                value_type: node.value_type,
                payload,
            }));
            next = next
                .checked_add(used)
                .ok_or(SealedSemanticDagError::ArithmeticOverflow)?;
        }
        for node in snapshot.nodes() {
            append_auxiliary(&mut cells, &node.payload);
        }
        if cells.len() != total as usize || next != total {
            return Err(SealedSemanticDagError::CorruptRegion);
        }
        Ok(RehydrationPlan {
            cells,
            cell_count: total,
            nodes: node_count,
            root: snapshot.root().get(),
        })
    }
}

fn validate_type_closure(
    snapshot: &SemanticDagSnapshot,
    expected: SemanticDagType,
    type_closure: &[SemanticDagType],
) -> Result<(), SealedSemanticDagError> {
    if snapshot.root_node().value_type != expected {
        return Err(SealedSemanticDagError::RootTypeMismatch);
    }
    if type_closure.is_empty()
        || type_closure.len() > MAX_MEMORY_WITNESSES
        || type_closure.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(SealedSemanticDagError::InvalidTypeClosure);
    }
    for node in snapshot.nodes() {
        if type_closure.binary_search(&node.value_type).is_err() {
            return Err(SealedSemanticDagError::UnresolvedType(node.value_type));
        }
    }
    Ok(())
}

fn auxiliary_cells(payload: &SemanticDagPayload) -> Result<u32, SealedSemanticDagError> {
    let count = match payload {
        SemanticDagPayload::String(bytes)
        | SemanticDagPayload::Path(bytes)
        | SemanticDagPayload::Bytes(bytes) => bytes.len().div_ceil(SEALED_DAG_BYTE_CHUNK),
        SemanticDagPayload::Product(fields) | SemanticDagPayload::Enum { fields, .. } => {
            fields.len()
        }
        _ => 0,
    };
    u32::try_from(count).map_err(|_| SealedSemanticDagError::ArithmeticOverflow)
}

fn append_auxiliary(cells: &mut Vec<SealedDagCell>, payload: &SemanticDagPayload) {
    match payload {
        SemanticDagPayload::String(bytes)
        | SemanticDagPayload::Path(bytes)
        | SemanticDagPayload::Bytes(bytes) => {
            for chunk in bytes.chunks(SEALED_DAG_BYTE_CHUNK) {
                let mut stored = [0_u8; SEALED_DAG_BYTE_CHUNK];
                stored[..chunk.len()].copy_from_slice(chunk);
                cells.push(SealedDagCell::Bytes {
                    length: chunk.len() as u8,
                    bytes: stored,
                });
            }
        }
        SemanticDagPayload::Product(fields) | SemanticDagPayload::Enum { fields, .. } => {
            cells.extend(fields.iter().copied().map(SealedDagCell::Child));
        }
        _ => {}
    }
}

fn planned_payload(
    payload: &SemanticDagPayload,
    first: u32,
) -> Result<(SealedDagNodePayload, u32), SealedSemanticDagError> {
    let used = auxiliary_cells(payload)?;
    let planned = match payload {
        SemanticDagPayload::Inline(value) => SealedDagNodePayload::Inline(*value),
        SemanticDagPayload::Static(value) => SealedDagNodePayload::Static(*value),
        SemanticDagPayload::String(bytes)
        | SemanticDagPayload::Path(bytes)
        | SemanticDagPayload::Bytes(bytes) => SealedDagNodePayload::Bytes {
            first,
            chunks: used,
            length: u32::try_from(bytes.len())
                .map_err(|_| SealedSemanticDagError::ArithmeticOverflow)?,
        },
        SemanticDagPayload::Product(_) => SealedDagNodePayload::Product {
            first,
            fields: used,
        },
        SemanticDagPayload::Enum { tag, .. } => SealedDagNodePayload::Enum {
            tag: *tag,
            first,
            fields: used,
        },
        SemanticDagPayload::EmptyList => SealedDagNodePayload::EmptyList,
        SemanticDagPayload::List { head, tail } => SealedDagNodePayload::List {
            head: *head,
            tail: *tail,
        },
    };
    Ok((planned, used))
}
