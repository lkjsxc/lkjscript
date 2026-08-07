use std::collections::VecDeque;

use super::super::value_runtime::{SemanticPayload, SemanticValue, StructuralValueError};
use super::{
    CheckedU64Range, LocalNodeId, StructuralImage, StructuralNodePayload, StructuralNodeRecord,
    TreeFacts,
};

impl StructuralImage {
    pub(crate) fn build(
        value: &SemanticValue,
        facts: TreeFacts,
    ) -> Result<Self, StructuralValueError> {
        let node_capacity =
            usize::try_from(facts.nodes).map_err(|_| StructuralValueError::ArithmeticOverflow)?;
        let field_count = facts
            .nodes
            .checked_sub(1)
            .ok_or(StructuralValueError::InvariantViolation)?;
        let field_capacity =
            usize::try_from(field_count).map_err(|_| StructuralValueError::ArithmeticOverflow)?;
        let blob_count = facts.bytes;
        let blob_capacity =
            usize::try_from(blob_count).map_err(|_| StructuralValueError::ArithmeticOverflow)?;
        let mut nodes = Vec::new();
        let mut fields = Vec::new();
        let mut blob = Vec::new();
        let mut queue = VecDeque::new();
        nodes.try_reserve_exact(node_capacity)?;
        fields.try_reserve_exact(field_capacity)?;
        blob.try_reserve_exact(blob_capacity)?;
        queue.try_reserve_exact(node_capacity)?;
        queue.push_back(value);
        let mut next_id = 1_u64;
        while let Some(value) = queue.pop_front() {
            let payload = match &value.payload {
                SemanticPayload::Inline(value) => StructuralNodePayload::Inline(*value),
                SemanticPayload::Static(value) => StructuralNodePayload::Static(*value),
                SemanticPayload::String(bytes)
                | SemanticPayload::Path(bytes)
                | SemanticPayload::Bytes(bytes)
                | SemanticPayload::ByteVector(bytes) => {
                    let start = u64::try_from(blob.len())
                        .map_err(|_| StructuralValueError::ArithmeticOverflow)?;
                    let length = u64::try_from(bytes.len())
                        .map_err(|_| StructuralValueError::ArithmeticOverflow)?;
                    blob.extend_from_slice(bytes);
                    StructuralNodePayload::Bytes(
                        CheckedU64Range::new(start, length)
                            .ok_or(StructuralValueError::ArithmeticOverflow)?,
                    )
                }
                SemanticPayload::Product(children) => StructuralNodePayload::Product(
                    append_children(children, &mut queue, &mut fields, &mut next_id)?,
                ),
                SemanticPayload::Enum {
                    tag,
                    active_payload,
                } => StructuralNodePayload::Enum {
                    tag: *tag,
                    fields: append_children(active_payload, &mut queue, &mut fields, &mut next_id)?,
                },
            };
            nodes.push(StructuralNodeRecord {
                value_type: value.value_type,
                payload,
            });
        }
        let image = Self {
            nodes,
            fields,
            blob,
        };
        image.validate(facts)?;
        Ok(image)
    }

    pub(crate) fn try_clone_flat(&self) -> Result<Self, StructuralValueError> {
        let mut nodes = Vec::new();
        let mut fields = Vec::new();
        let mut blob = Vec::new();
        nodes.try_reserve_exact(self.nodes.len())?;
        fields.try_reserve_exact(self.fields.len())?;
        blob.try_reserve_exact(self.blob.len())?;
        nodes.extend_from_slice(&self.nodes);
        fields.extend_from_slice(&self.fields);
        blob.extend_from_slice(&self.blob);
        Ok(Self {
            nodes,
            fields,
            blob,
        })
    }
}

fn append_children<'a>(
    children: &'a [SemanticValue],
    queue: &mut VecDeque<&'a SemanticValue>,
    fields: &mut Vec<LocalNodeId>,
    next_id: &mut u64,
) -> Result<CheckedU64Range, StructuralValueError> {
    let start =
        u64::try_from(fields.len()).map_err(|_| StructuralValueError::ArithmeticOverflow)?;
    for child in children {
        let id = *next_id;
        *next_id = next_id
            .checked_add(1)
            .ok_or(StructuralValueError::ArithmeticOverflow)?;
        fields.push(LocalNodeId::new(id));
        queue.push_back(child);
    }
    let length =
        u64::try_from(children.len()).map_err(|_| StructuralValueError::ArithmeticOverflow)?;
    CheckedU64Range::new(start, length).ok_or(StructuralValueError::ArithmeticOverflow)
}

pub(crate) fn prepare_discard(
    facts: TreeFacts,
) -> Result<Vec<SemanticValue>, StructuralValueError> {
    let mut stack = Vec::new();
    stack
        .try_reserve_exact(
            usize::try_from(facts.nodes).map_err(|_| StructuralValueError::AllocationFailed)?,
        )
        .map_err(|_| StructuralValueError::AllocationFailed)?;
    Ok(stack)
}

pub(crate) fn discard_semantic(mut value: SemanticValue, stack: &mut Vec<SemanticValue>) {
    loop {
        match value.payload {
            SemanticPayload::Product(fields)
            | SemanticPayload::Enum {
                active_payload: fields,
                ..
            } => stack.extend(fields),
            SemanticPayload::Inline(_)
            | SemanticPayload::Static(_)
            | SemanticPayload::String(_)
            | SemanticPayload::Path(_)
            | SemanticPayload::Bytes(_)
            | SemanticPayload::ByteVector(_) => {}
        }
        let Some(next) = stack.pop() else {
            break;
        };
        value = next;
    }
}
