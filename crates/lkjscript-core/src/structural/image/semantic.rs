use super::super::value_runtime::{
    InlineStructuralValue, SemanticPayload, SemanticValue, StructuralKind, StructuralValueError,
    StructuralValueLimit, StructuralValueRuntimeLimits,
};
use super::TreeFacts;

pub(crate) fn semantic_facts(
    value: &SemanticValue,
    limits: StructuralValueRuntimeLimits,
) -> Result<TreeFacts, StructuralValueError> {
    let mut stack = Vec::new();
    stack
        .try_reserve(1)
        .map_err(|_| StructuralValueError::AllocationFailed)?;
    stack.push((value, 1_u16));
    let mut facts = TreeFacts::default();
    while let Some((node, depth)) = stack.pop() {
        if depth > limits.max_tree_depth {
            return Err(StructuralValueError::LimitExceeded(
                StructuralValueLimit::TreeDepth,
            ));
        }
        facts.nodes = facts
            .nodes
            .checked_add(1)
            .ok_or(StructuralValueError::ArithmeticOverflow)?;
        if facts.nodes > limits.max_tree_nodes {
            return Err(StructuralValueError::LimitExceeded(
                StructuralValueLimit::TreeNodes,
            ));
        }
        match &node.payload {
            SemanticPayload::Inline(inline) => {
                let expected = match inline {
                    InlineStructuralValue::Unit => StructuralKind::Unit,
                    InlineStructuralValue::Bool(_) => StructuralKind::Bool,
                    InlineStructuralValue::I64(_) => StructuralKind::I64,
                    InlineStructuralValue::F64Bits(_) => StructuralKind::F64,
                };
                require_kind(node.value_type.kind, expected)?;
            }
            SemanticPayload::Static(_) => {
                require_kind(node.value_type.kind, StructuralKind::Static)?;
            }
            SemanticPayload::String(bytes) => {
                require_kind(node.value_type.kind, StructuralKind::String)?;
                std::str::from_utf8(bytes).map_err(|_| StructuralValueError::InvalidUtf8)?;
                add_bytes(&mut facts, bytes.len(), StructuralKind::String, limits)?;
            }
            SemanticPayload::Path(bytes) => {
                require_kind(node.value_type.kind, StructuralKind::Path)?;
                if bytes.first() != Some(&b'/') || bytes.contains(&0) {
                    return Err(StructuralValueError::InvalidPath);
                }
                add_bytes(&mut facts, bytes.len(), StructuralKind::Path, limits)?;
            }
            SemanticPayload::Bytes(bytes) => {
                require_kind(node.value_type.kind, StructuralKind::Bytes)?;
                add_bytes(&mut facts, bytes.len(), StructuralKind::Bytes, limits)?;
            }
            SemanticPayload::ByteVector(bytes) => {
                require_kind(node.value_type.kind, StructuralKind::ByteVector)?;
                add_bytes(&mut facts, bytes.len(), StructuralKind::ByteVector, limits)?;
            }
            SemanticPayload::Product(fields) => {
                require_kind(node.value_type.kind, StructuralKind::Product)?;
                push_fields(&mut stack, fields, depth, facts.nodes, limits)?;
            }
            SemanticPayload::Enum { active_payload, .. } => {
                require_kind(node.value_type.kind, StructuralKind::Enum)?;
                push_fields(&mut stack, active_payload, depth, facts.nodes, limits)?;
            }
        }
    }
    Ok(facts)
}

fn push_fields<'a>(
    stack: &mut Vec<(&'a SemanticValue, u16)>,
    fields: &'a [SemanticValue],
    depth: u16,
    visited: u32,
    limits: StructuralValueRuntimeLimits,
) -> Result<(), StructuralValueError> {
    if fields.len() > limits.max_fields {
        return Err(StructuralValueError::LimitExceeded(
            StructuralValueLimit::Fields,
        ));
    }
    let pending =
        u32::try_from(stack.len()).map_err(|_| StructuralValueError::ArithmeticOverflow)?;
    let added =
        u32::try_from(fields.len()).map_err(|_| StructuralValueError::ArithmeticOverflow)?;
    if visited
        .checked_add(pending)
        .and_then(|total| total.checked_add(added))
        .is_none_or(|total| total > limits.max_tree_nodes)
    {
        return Err(StructuralValueError::LimitExceeded(
            StructuralValueLimit::TreeNodes,
        ));
    }
    stack
        .try_reserve(fields.len())
        .map_err(|_| StructuralValueError::AllocationFailed)?;
    let next = depth
        .checked_add(1)
        .ok_or(StructuralValueError::ArithmeticOverflow)?;
    for field in fields.iter().rev() {
        stack.push((field, next));
    }
    Ok(())
}

fn add_bytes(
    facts: &mut TreeFacts,
    length: usize,
    kind: StructuralKind,
    limits: StructuralValueRuntimeLimits,
) -> Result<(), StructuralValueError> {
    let length = u64::try_from(length).map_err(|_| StructuralValueError::ArithmeticOverflow)?;
    facts.bytes = facts
        .bytes
        .checked_add(length)
        .ok_or(StructuralValueError::ArithmeticOverflow)?;
    if facts.bytes > limits.max_payload_bytes {
        return Err(StructuralValueError::LimitExceeded(
            StructuralValueLimit::PayloadBytes,
        ));
    }
    if kind == StructuralKind::String {
        facts.string_bytes = facts
            .string_bytes
            .checked_add(length)
            .ok_or(StructuralValueError::ArithmeticOverflow)?;
    } else if kind == StructuralKind::Path {
        facts.path_bytes = facts
            .path_bytes
            .checked_add(length)
            .ok_or(StructuralValueError::ArithmeticOverflow)?;
    }
    Ok(())
}

pub(crate) fn require_kind(
    actual: StructuralKind,
    expected: StructuralKind,
) -> Result<(), StructuralValueError> {
    (actual == expected)
        .then_some(())
        .ok_or(StructuralValueError::WrongPayloadKind)
}
