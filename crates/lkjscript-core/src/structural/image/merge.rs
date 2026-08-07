use super::super::value_runtime::{StructuralType, StructuralValueError};
use super::{
    CheckedU64Range, LocalNodeId, StructuralImage, StructuralNodePayload, StructuralNodeRecord,
    TreeFacts,
};

impl StructuralImage {
    pub(crate) fn merge(
        value_type: StructuralType,
        enum_tag: Option<u64>,
        children: &[&StructuralImage],
        facts: TreeFacts,
    ) -> Result<Self, StructuralValueError> {
        let nodes_len =
            usize::try_from(facts.nodes).map_err(|_| StructuralValueError::ArithmeticOverflow)?;
        let fields_len = nodes_len
            .checked_sub(1)
            .ok_or(StructuralValueError::InvariantViolation)?;
        let blob_len =
            usize::try_from(facts.bytes).map_err(|_| StructuralValueError::ArithmeticOverflow)?;
        let mut nodes = Vec::new();
        let mut fields = Vec::new();
        let mut blob = Vec::new();
        nodes.try_reserve_exact(nodes_len)?;
        fields.try_reserve_exact(fields_len)?;
        blob.try_reserve_exact(blob_len)?;
        let root_fields = CheckedU64Range::new(
            0,
            u64::try_from(children.len()).map_err(|_| StructuralValueError::ArithmeticOverflow)?,
        )
        .ok_or(StructuralValueError::ArithmeticOverflow)?;
        nodes.push(StructuralNodeRecord {
            value_type,
            payload: match enum_tag {
                Some(tag) => StructuralNodePayload::Enum {
                    tag,
                    fields: root_fields,
                },
                None => StructuralNodePayload::Product(root_fields),
            },
        });
        let mut node_offset = 1_u64;
        for child in children {
            fields.push(LocalNodeId::new(node_offset));
            node_offset = node_offset
                .checked_add(child.node_count())
                .ok_or(StructuralValueError::ArithmeticOverflow)?;
        }
        node_offset = 1;
        let mut field_offset =
            u64::try_from(children.len()).map_err(|_| StructuralValueError::ArithmeticOverflow)?;
        let mut blob_offset = 0_u64;
        for child in children {
            for record in &child.nodes {
                nodes.push(StructuralNodeRecord {
                    value_type: record.value_type,
                    payload: shift_payload(&record.payload, field_offset, blob_offset)?,
                });
            }
            for id in &child.fields {
                fields.push(LocalNodeId::new(
                    id.get()
                        .checked_add(node_offset)
                        .ok_or(StructuralValueError::ArithmeticOverflow)?,
                ));
            }
            blob.extend_from_slice(&child.blob);
            node_offset = node_offset
                .checked_add(child.node_count())
                .ok_or(StructuralValueError::ArithmeticOverflow)?;
            field_offset = field_offset
                .checked_add(child.field_cell_count())
                .ok_or(StructuralValueError::ArithmeticOverflow)?;
            blob_offset = blob_offset
                .checked_add(child.blob_len())
                .ok_or(StructuralValueError::ArithmeticOverflow)?;
        }
        let image = Self {
            nodes,
            fields,
            blob,
        };
        image.validate(facts)?;
        Ok(image)
    }
}

fn shift_payload(
    payload: &StructuralNodePayload,
    field_offset: u64,
    blob_offset: u64,
) -> Result<StructuralNodePayload, StructuralValueError> {
    match payload {
        StructuralNodePayload::Inline(value) => Ok(StructuralNodePayload::Inline(*value)),
        StructuralNodePayload::Static(value) => Ok(StructuralNodePayload::Static(*value)),
        StructuralNodePayload::Bytes(range) => Ok(StructuralNodePayload::Bytes(shift_range(
            *range,
            blob_offset,
        )?)),
        StructuralNodePayload::Product(range) => Ok(StructuralNodePayload::Product(shift_range(
            *range,
            field_offset,
        )?)),
        StructuralNodePayload::Enum { tag, fields } => Ok(StructuralNodePayload::Enum {
            tag: *tag,
            fields: shift_range(*fields, field_offset)?,
        }),
    }
}

fn shift_range(
    range: CheckedU64Range,
    offset: u64,
) -> Result<CheckedU64Range, StructuralValueError> {
    let start = range
        .start()
        .checked_add(offset)
        .ok_or(StructuralValueError::ArithmeticOverflow)?;
    CheckedU64Range::new(start, range.len()).ok_or(StructuralValueError::ArithmeticOverflow)
}
