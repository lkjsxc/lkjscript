use super::super::value_runtime::{InlineStructuralValue, StructuralKind, StructuralValueError};
use super::{require_kind, StructuralImage, StructuralNodePayload, TreeFacts};

impl StructuralImage {
    pub(crate) fn validate(&self, expected: TreeFacts) -> Result<(), StructuralValueError> {
        if self.nodes.is_empty() {
            return Err(StructuralValueError::InvariantViolation);
        }
        let node_count = u64::try_from(self.nodes.len())
            .map_err(|_| StructuralValueError::ArithmeticOverflow)?;
        if self.fields.len() != self.nodes.len() - 1 {
            return Err(StructuralValueError::InvariantViolation);
        }
        let mut parent_seen = filled(self.nodes.len(), false)?;
        let mut field_used = filled(self.fields.len(), false)?;
        let mut byte_used = filled(self.blob.len(), false)?;
        let mut facts = TreeFacts {
            nodes: node_count,
            ..TreeFacts::default()
        };
        for (index, node) in self.nodes.iter().enumerate() {
            let parent =
                u64::try_from(index).map_err(|_| StructuralValueError::ArithmeticOverflow)?;
            match &node.payload {
                StructuralNodePayload::Inline(value) => {
                    let expected = match value {
                        InlineStructuralValue::Unit => StructuralKind::Unit,
                        InlineStructuralValue::Bool(_) => StructuralKind::Bool,
                        InlineStructuralValue::I64(_) => StructuralKind::I64,
                        InlineStructuralValue::F64Bits(_) => StructuralKind::F64,
                    };
                    require_kind(node.value_type.kind, expected)?;
                }
                StructuralNodePayload::Static(_) => {
                    require_kind(node.value_type.kind, StructuralKind::Static)?;
                }
                StructuralNodePayload::Bytes(range) => {
                    validate_byte_kind(node.value_type.kind)?;
                    let bytes = Self::range(&self.blob, *range)
                        .ok_or(StructuralValueError::InvariantViolation)?;
                    mark(&mut byte_used, range.start(), range.end())?;
                    validate_bytes(node.value_type.kind, bytes)?;
                    add_bytes(&mut facts, node.value_type.kind, bytes.len())?;
                }
                StructuralNodePayload::Product(range) => {
                    require_kind(node.value_type.kind, StructuralKind::Product)?;
                    validate_children(self, *range, parent, &mut parent_seen, &mut field_used)?;
                }
                StructuralNodePayload::Enum { fields, .. } => {
                    require_kind(node.value_type.kind, StructuralKind::Enum)?;
                    validate_children(self, *fields, parent, &mut parent_seen, &mut field_used)?;
                }
            }
        }
        if parent_seen[0]
            || parent_seen[1..].iter().any(|seen| !seen)
            || field_used.iter().any(|used| !used)
            || byte_used.iter().any(|used| !used)
            || facts != expected
        {
            return Err(StructuralValueError::InvariantViolation);
        }
        Ok(())
    }
}

fn validate_children(
    image: &StructuralImage,
    range: super::CheckedU64Range,
    parent: u64,
    parent_seen: &mut [bool],
    used: &mut [bool],
) -> Result<(), StructuralValueError> {
    let children = StructuralImage::range(&image.fields, range)
        .ok_or(StructuralValueError::InvariantViolation)?;
    mark(used, range.start(), range.end())?;
    for child in children {
        let index = child
            .index()
            .ok_or(StructuralValueError::InvariantViolation)?;
        if child.get() <= parent || index >= image.nodes.len() {
            return Err(StructuralValueError::InvariantViolation);
        }
        if std::mem::replace(&mut parent_seen[index], true) {
            return Err(StructuralValueError::InvariantViolation);
        }
    }
    Ok(())
}

fn validate_byte_kind(kind: StructuralKind) -> Result<(), StructuralValueError> {
    match kind {
        StructuralKind::String
        | StructuralKind::Path
        | StructuralKind::Bytes
        | StructuralKind::ByteVector => Ok(()),
        _ => Err(StructuralValueError::WrongPayloadKind),
    }
}

fn validate_bytes(kind: StructuralKind, bytes: &[u8]) -> Result<(), StructuralValueError> {
    if kind == StructuralKind::String {
        std::str::from_utf8(bytes).map_err(|_| StructuralValueError::InvalidUtf8)?;
    } else if kind == StructuralKind::Path && (bytes.first() != Some(&b'/') || bytes.contains(&0)) {
        return Err(StructuralValueError::InvalidPath);
    }
    Ok(())
}

fn add_bytes(
    facts: &mut TreeFacts,
    kind: StructuralKind,
    length: usize,
) -> Result<(), StructuralValueError> {
    let length = u64::try_from(length).map_err(|_| StructuralValueError::ArithmeticOverflow)?;
    facts.bytes = facts
        .bytes
        .checked_add(length)
        .ok_or(StructuralValueError::ArithmeticOverflow)?;
    facts.string_bytes = facts
        .string_bytes
        .checked_add(u64::from(kind == StructuralKind::String) * length)
        .ok_or(StructuralValueError::ArithmeticOverflow)?;
    facts.path_bytes = facts
        .path_bytes
        .checked_add(u64::from(kind == StructuralKind::Path) * length)
        .ok_or(StructuralValueError::ArithmeticOverflow)?;
    Ok(())
}

fn mark(used: &mut [bool], start: u64, end: u64) -> Result<(), StructuralValueError> {
    let start = usize::try_from(start).map_err(|_| StructuralValueError::InvariantViolation)?;
    let end = usize::try_from(end).map_err(|_| StructuralValueError::InvariantViolation)?;
    let range = used
        .get_mut(start..end)
        .ok_or(StructuralValueError::InvariantViolation)?;
    if range.iter().any(|value| *value) {
        return Err(StructuralValueError::InvariantViolation);
    }
    range.fill(true);
    Ok(())
}

fn filled<T: Clone>(length: usize, value: T) -> Result<Vec<T>, StructuralValueError> {
    let mut values = Vec::new();
    values.try_reserve_exact(length)?;
    values.resize(length, value);
    Ok(values)
}
