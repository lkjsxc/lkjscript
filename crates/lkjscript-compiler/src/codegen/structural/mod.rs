use crate::codegen::*;

mod requirements;
mod witnesses;
use witnesses::install_memory_witnesses;

include!("installation.rs");

pub(in crate::codegen) fn structural_owner_representation(
    chunk: &Chunk,
    ty: &SsaType,
) -> Option<BytecodeStructuralRepresentationId> {
    structural_representation(chunk, ty, BytecodeStructuralValueCategory::Owner)
}

pub(in crate::codegen) fn structural_view_representation(
    chunk: &Chunk,
    ty: &SsaType,
) -> Option<BytecodeStructuralRepresentationId> {
    structural_representation(chunk, ty, BytecodeStructuralValueCategory::View)
}

pub(in crate::codegen) fn structural_destination(
    chunk: &Chunk,
    representation: lkjscript_ir::StructuralRepresentationId,
    active_variant: Option<lkjscript_ir::VariantId>,
) -> Result<StructuralDestinationId> {
    let representation = BytecodeStructuralRepresentationId::new(representation.raw());
    let active_variant = active_variant.map(|variant| BytecodeVariantId::new(variant.bytes()));
    chunk
        .structural_destinations
        .iter()
        .find(|item| item.representation == representation && item.active_variant == active_variant)
        .map(|item| item.id)
        .ok_or_else(|| Error::msg("SSA destination has no exact bytecode metadata"))
}

pub(in crate::codegen) fn intern_destination_field(
    chunk: &mut Chunk,
    destination: StructuralDestinationId,
    field: u16,
) -> Result<u16> {
    let reference = StructuralDestinationFieldRef { destination, field };
    if let Some(index) = chunk
        .structural_destination_fields
        .iter()
        .position(|item| *item == reference)
    {
        return u16::try_from(index)
            .map_err(|_| Error::msg("structural destination-field index exceeds u16"));
    }
    let index = u16::try_from(chunk.structural_destination_fields.len())
        .map_err(|_| Error::msg("structural destination-field table exceeds u16"))?;
    chunk.structural_destination_fields.push(reference);
    Ok(index)
}

pub(in crate::codegen) fn intern_aggregate_field(
    chunk: &mut Chunk,
    representation: lkjscript_ir::StructuralRepresentationId,
    field: u16,
    result: &SsaType,
) -> Result<u16> {
    intern_aggregate_field_for_representation(
        chunk,
        BytecodeStructuralRepresentationId::new(representation.raw()),
        field,
        result,
    )
}

pub(in crate::codegen) fn intern_aggregate_field_for_representation(
    chunk: &mut Chunk,
    representation: BytecodeStructuralRepresentationId,
    field: u16,
    result: &SsaType,
) -> Result<u16> {
    let metadata = chunk
        .structural_representations
        .get(representation.index())
        .ok_or_else(|| Error::msg("aggregate field representation is missing"))?;
    let layout = chunk
        .structural_layouts
        .get(metadata.layout.index())
        .ok_or_else(|| Error::msg("aggregate field layout is missing"))?;
    let active_variant = match &layout.kind {
        BytecodeStructuralLayoutKind::Enum { variants, .. } if variants.len() == 1 => {
            Some(variants[0].variant)
        }
        BytecodeStructuralLayoutKind::Enum { .. } => {
            return Err(Error::msg(
                "aggregate enum field borrow requires exact active payload metadata",
            ))
        }
        BytecodeStructuralLayoutKind::String
        | BytecodeStructuralLayoutKind::Path
        | BytecodeStructuralLayoutKind::Product { .. } => None,
    };
    let reference = StructuralAggregateFieldRef {
        representation,
        active_variant,
        field,
        result: structural_field_from_chunk(chunk, result)?,
    };
    if let Some(index) = chunk
        .structural_aggregate_fields
        .iter()
        .position(|item| *item == reference)
    {
        return u16::try_from(index)
            .map_err(|_| Error::msg("structural aggregate-field index exceeds u16"));
    }
    let index = u16::try_from(chunk.structural_aggregate_fields.len())
        .map_err(|_| Error::msg("structural aggregate-field table exceeds u16"))?;
    chunk.structural_aggregate_fields.push(reference);
    Ok(index)
}

pub(in crate::codegen) fn intern_payload(
    chunk: &mut Chunk,
    representation: BytecodeStructuralRepresentationId,
    variant: lkjscript_ir::VariantId,
    result: &SsaType,
) -> Result<u16> {
    let reference = StructuralPayloadRef {
        representation,
        variant: BytecodeVariantId::new(variant.bytes()),
        result: structural_field_from_chunk(chunk, result)?,
    };
    if let Some(index) = chunk
        .structural_payloads
        .iter()
        .position(|item| *item == reference)
    {
        return u16::try_from(index)
            .map_err(|_| Error::msg("structural payload index exceeds u16"));
    }
    let index = u16::try_from(chunk.structural_payloads.len())
        .map_err(|_| Error::msg("structural payload table exceeds u16"))?;
    chunk.structural_payloads.push(reference);
    Ok(index)
}

include!("metadata.rs");
