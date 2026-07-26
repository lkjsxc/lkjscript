use crate::*;

#[derive(Clone, Copy)]
pub(crate) struct EnumFacts {
    pub(super) layout: lkjscript_core::RuntimeLayoutId,
    pub(super) physical_tag: u16,
    pub(super) field_count: usize,
    pub(super) field_index: Option<usize>,
}

pub(crate) fn enum_facts(
    enums: &[lkjscript_ir::EnumMetadata],
    operation: &HeapOperation,
) -> Result<EnumFacts, &'static str> {
    let (enum_id, variant_id, layout_id) =
        operation_ids(operation).ok_or("enum operation metadata is malformed")?;
    let definition = enums
        .iter()
        .find(|item| item.id.bytes() == enum_id)
        .ok_or("enum metadata identity is invalid")?;
    if definition.layout.identity.bytes() != layout_id {
        return Err("enum operation layout identity mismatch");
    }
    let selected = definition
        .variants
        .iter()
        .find(|item| item.id.bytes() == variant_id)
        .ok_or("enum variant identity is invalid")?;
    let (physical_tag, field_index) = operation_facts(operation, definition, selected)?;
    Ok(EnumFacts {
        layout: lkjscript_core::RuntimeLayoutId::new(definition.layout.identity.bytes()),
        physical_tag,
        field_count: selected.fields.len(),
        field_index,
    })
}

fn operation_facts(
    operation: &HeapOperation,
    definition: &lkjscript_ir::EnumMetadata,
    selected: &lkjscript_ir::EnumVariantMetadata,
) -> Result<(u16, Option<usize>), &'static str> {
    match operation {
        HeapOperation::EnumValue {
            physical_tag,
            substitutions,
            fields,
            ..
        } if *physical_tag == selected.physical_tag
            && substitutions.len() == definition.type_parameters.len()
            && usize::from(*fields) == selected.fields.len() =>
        {
            Ok((*physical_tag, None))
        }
        HeapOperation::EnumIsVariant { physical_tag, .. }
            if *physical_tag == selected.physical_tag =>
        {
            Ok((*physical_tag, None))
        }
        HeapOperation::EnumField {
            field,
            physical_tag,
            field_index,
            ..
        } if *physical_tag == selected.physical_tag
            && selected
                .fields
                .get(usize::from(*field_index))
                .is_some_and(|item| item.id.bytes() == *field) =>
        {
            Ok((*physical_tag, Some(usize::from(*field_index))))
        }
        _ => Err("enum operation tag/field metadata mismatch"),
    }
}

pub(super) fn operation_ids(operation: &HeapOperation) -> Option<([u8; 32], [u8; 32], [u8; 32])> {
    match operation {
        HeapOperation::EnumValue {
            enum_id,
            variant,
            layout,
            ..
        }
        | HeapOperation::EnumIsVariant {
            enum_id,
            variant,
            layout,
            ..
        }
        | HeapOperation::EnumField {
            enum_id,
            variant,
            layout,
            ..
        } => Some((*enum_id, *variant, *layout)),
        _ => None,
    }
}
