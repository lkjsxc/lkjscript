use lkjscript_core::{
    SemanticPayload, SemanticValue, StructuralDestinationMetadata, StructuralFieldMetadata,
    StructuralFieldRoute, StructuralKind, StructuralLayoutKind,
};

use super::*;

pub(super) fn dispatch<J: RuntimeTier>(vm: &mut Vm<'_, J>, op: lkjscript_core::Op) -> Result<()> {
    match op {
        lkjscript_core::Op::StructuralDestinationCreate => create(vm),
        lkjscript_core::Op::StructuralDestinationFieldInit => initialize(vm),
        lkjscript_core::Op::StructuralDestinationFinish => finish(vm),
        lkjscript_core::Op::StructuralDestinationAbort => abort(vm),
        _ => Err(Error::msg(
            "structural destination opcode dispatch mismatch",
        )),
    }
}

fn create<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    charge_construction(vm)?;
    let id = StructuralDestinationId::new(vm.read_u16()?);
    let metadata = destination_metadata(vm.chunk, id)?.clone();
    let value_type = representation_type(
        vm.chunk,
        metadata.representation,
        StructuralValueCategory::Destination,
    )?;
    let field_types = metadata
        .fields
        .iter()
        .map(|field| {
            field
                .runtime_type
                .ok_or_else(|| Error::msg("structural destination field has no value-runtime type"))
        })
        .collect::<Result<Vec<_>>>()?;
    let layout = representation(vm.chunk, metadata.representation)?;
    let layout = vm
        .chunk
        .structural_layouts()
        .get(layout.layout.index())
        .filter(|item| item.id == layout.layout)
        .ok_or_else(|| Error::msg("structural destination layout is stale"))?;
    let key = match (&layout.kind, metadata.active_variant) {
        (StructuralLayoutKind::Product { .. }, None) => invocation_mut(vm)?
            .runtime
            .begin_product(value_type, field_types),
        (StructuralLayoutKind::Enum { variants, .. }, Some(active)) => {
            let tag = variants
                .iter()
                .find(|variant| variant.variant == active)
                .map(|variant| variant.physical_tag)
                .ok_or_else(|| Error::msg("structural destination active variant is stale"))?;
            invocation_mut(vm)?
                .runtime
                .begin_enum(value_type, tag, field_types)
        }
        (StructuralLayoutKind::String | StructuralLayoutKind::Path, None) => {
            return Err(Error::msg(
                "leaf structural destination is not constructible",
            ));
        }
        _ => {
            return Err(Error::msg(
                "structural destination shape does not match exact metadata",
            ));
        }
    }
    .map_err(map_value_error)?;
    let value = invocation_mut(vm)?.register_destination(key, id, value_type)?;
    vm.push(value);
    Ok(())
}

fn initialize<J: RuntimeTier>(vm: &mut Vm<'_, J>) -> Result<()> {
    let reference_index = usize::from(vm.read_u16()?);
    let reference = vm
        .chunk
        .structural_destination_fields()
        .get(reference_index)
        .copied()
        .ok_or_else(|| Error::msg("structural destination-field reference is stale"))?;
    let source = vm.pop()?;
    let destination_value = vm.pop()?;
    let (destination_key, destination_record) = invocation(vm)?.destination(destination_value)?;
    if destination_record.destination != reference.destination {
        return Err(Error::msg(
            "structural destination-field reference does not match its value",
        ));
    }
    let metadata = destination_metadata(vm.chunk, reference.destination)?;
    let field = *metadata
        .fields
        .get(usize::from(reference.field))
        .ok_or_else(|| Error::msg("structural destination field is out of range"))?;
    match field.route {
        StructuralFieldRoute::Copy => invocation_mut(vm)?
            .runtime
            .initialize_value(destination_key, reference.field, source)
            .map_err(map_value_error)?,
        StructuralFieldRoute::Structural(_) => {
            initialize_structural_owner(vm, destination_key, reference.field, source, field)?;
        }
        StructuralFieldRoute::Unique => {
            initialize_unique_owner(vm, destination_key, reference.field, source, field)?;
        }
        StructuralFieldRoute::Resource => {
            return Err(Error::msg(
                "structural destination resource fields are not executable in the VM",
            ));
        }
        StructuralFieldRoute::LegacyHeap => {
            return Err(Error::msg(
                "structural destination cannot bridge a legacy heap field",
            ));
        }
    }
    vm.push(destination_value);
    Ok(())
}

include!("destination/initialize.rs");
include!("destination/finalize.rs");
