use super::super::value_runtime::{
    InlineStructuralValue, SemanticPayload, SemanticValue, StructuralKind, StructuralValueError,
};
use super::TreeFacts;

pub(crate) fn semantic_facts(value: &SemanticValue) -> Result<TreeFacts, StructuralValueError> {
    let mut stack = Vec::new();
    stack
        .try_reserve(1)
        .map_err(|_| StructuralValueError::AllocationFailed)?;
    stack.push(value);
    let mut facts = TreeFacts::default();
    while let Some(node) = stack.pop() {
        facts.nodes = facts
            .nodes
            .checked_add(1)
            .ok_or(StructuralValueError::ArithmeticOverflow)?;
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
                add_bytes(&mut facts, bytes.len(), StructuralKind::String)?;
            }
            SemanticPayload::Path(bytes) => {
                require_kind(node.value_type.kind, StructuralKind::Path)?;
                if bytes.first() != Some(&b'/') || bytes.contains(&0) {
                    return Err(StructuralValueError::InvalidPath);
                }
                add_bytes(&mut facts, bytes.len(), StructuralKind::Path)?;
            }
            SemanticPayload::Bytes(bytes) => {
                require_kind(node.value_type.kind, StructuralKind::Bytes)?;
                add_bytes(&mut facts, bytes.len(), StructuralKind::Bytes)?;
            }
            SemanticPayload::ByteVector(bytes) => {
                require_kind(node.value_type.kind, StructuralKind::ByteVector)?;
                add_bytes(&mut facts, bytes.len(), StructuralKind::ByteVector)?;
            }
            SemanticPayload::Product(fields) => {
                require_kind(node.value_type.kind, StructuralKind::Product)?;
                push_fields(&mut stack, fields)?;
            }
            SemanticPayload::Enum { active_payload, .. } => {
                require_kind(node.value_type.kind, StructuralKind::Enum)?;
                push_fields(&mut stack, active_payload)?;
            }
        }
    }
    Ok(facts)
}

fn push_fields<'a>(
    stack: &mut Vec<&'a SemanticValue>,
    fields: &'a [SemanticValue],
) -> Result<(), StructuralValueError> {
    stack
        .try_reserve(fields.len())
        .map_err(|_| StructuralValueError::AllocationFailed)?;
    stack.extend(fields.iter().rev());
    Ok(())
}

fn add_bytes(
    facts: &mut TreeFacts,
    length: usize,
    kind: StructuralKind,
) -> Result<(), StructuralValueError> {
    let length = u64::try_from(length).map_err(|_| StructuralValueError::ArithmeticOverflow)?;
    facts.bytes = facts
        .bytes
        .checked_add(length)
        .ok_or(StructuralValueError::ArithmeticOverflow)?;
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
