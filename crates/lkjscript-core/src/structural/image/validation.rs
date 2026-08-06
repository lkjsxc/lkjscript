use super::super::value_runtime::{
    InlineStructuralValue, StructuralKind, StructuralValueError, StructuralValueLimit,
    StructuralValueRuntimeLimits,
};
use super::{require_kind, StructuralImage, StructuralNodePayload, TreeFacts};

impl StructuralImage {
    pub(crate) fn validate(
        &self,
        limits: StructuralValueRuntimeLimits,
        expected: TreeFacts,
    ) -> Result<(), StructuralValueError> {
        if self.nodes.is_empty() {
            return Err(StructuralValueError::InvariantViolation);
        }
        let node_count = u32::try_from(self.nodes.len())
            .map_err(|_| StructuralValueError::ArithmeticOverflow)?;
        if node_count > limits.max_tree_nodes {
            return Err(StructuralValueError::LimitExceeded(
                StructuralValueLimit::TreeNodes,
            ));
        }
        if self.fields.len() != self.nodes.len() - 1 {
            return Err(StructuralValueError::InvariantViolation);
        }
        let mut parents = filled(self.nodes.len(), 0_u16)?;
        let mut depths = filled(self.nodes.len(), 0_u16)?;
        let mut field_used = filled(self.fields.len(), false)?;
        let mut byte_used = filled(self.blob.len(), false)?;
        depths[0] = 1_u16;
        let mut facts = TreeFacts {
            nodes: node_count,
            ..TreeFacts::default()
        };
        for (index, node) in self.nodes.iter().enumerate() {
            let parent =
                u32::try_from(index).map_err(|_| StructuralValueError::ArithmeticOverflow)?;
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
                    add_bytes(&mut facts, node.value_type.kind, bytes.len(), limits)?;
                }
                StructuralNodePayload::Product(range) => {
                    require_kind(node.value_type.kind, StructuralKind::Product)?;
                    validate_children(
                        self,
                        *range,
                        parent,
                        &mut parents,
                        &mut depths,
                        &mut field_used,
                        limits,
                    )?;
                }
                StructuralNodePayload::Enum { fields, .. } => {
                    require_kind(node.value_type.kind, StructuralKind::Enum)?;
                    validate_children(
                        self,
                        *fields,
                        parent,
                        &mut parents,
                        &mut depths,
                        &mut field_used,
                        limits,
                    )?;
                }
            }
        }
        if parents[0] != 0
            || parents[1..].iter().any(|count| *count != 1)
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
    range: super::CheckedU32Range,
    parent: u32,
    parents: &mut [u16],
    depths: &mut [u16],
    used: &mut [bool],
    limits: StructuralValueRuntimeLimits,
) -> Result<(), StructuralValueError> {
    if usize::try_from(range.len()).map_or(true, |fields| fields > limits.max_fields) {
        return Err(StructuralValueError::LimitExceeded(
            StructuralValueLimit::Fields,
        ));
    }
    let children = StructuralImage::range(&image.fields, range)
        .ok_or(StructuralValueError::InvariantViolation)?;
    mark(used, range.start(), range.end())?;
    let depth = depths[parent as usize]
        .checked_add(1)
        .ok_or(StructuralValueError::ArithmeticOverflow)?;
    if depth > limits.max_tree_depth && !children.is_empty() {
        return Err(StructuralValueError::LimitExceeded(
            StructuralValueLimit::TreeDepth,
        ));
    }
    for child in children {
        if child.get() <= parent || child.get() as usize >= image.nodes.len() {
            return Err(StructuralValueError::InvariantViolation);
        }
        let index = child.get() as usize;
        parents[index] = parents[index]
            .checked_add(1)
            .ok_or(StructuralValueError::ArithmeticOverflow)?;
        if parents[index] != 1 || (depths[index] != 0 && depths[index] != depth) {
            return Err(StructuralValueError::InvariantViolation);
        }
        depths[index] = depth;
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
    facts.string_bytes += u64::from(kind == StructuralKind::String) * length;
    facts.path_bytes += u64::from(kind == StructuralKind::Path) * length;
    Ok(())
}

fn mark(used: &mut [bool], start: u32, end: u32) -> Result<(), StructuralValueError> {
    let range = used
        .get_mut(start as usize..end as usize)
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
