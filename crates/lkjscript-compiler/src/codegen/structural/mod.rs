use crate::codegen::*;

mod requirements;
mod witnesses;
use witnesses::install_memory_witnesses;

include!("installation.rs");
include!("installation/destinations.rs");

pub(in crate::codegen) fn structural_owner_representation(
    chunk: &Chunk,
    ty: &SsaType,
) -> Option<BytecodeStructuralRepresentationId> {
    structural_representation(
        chunk,
        ty,
        BytecodeStructuralValueCategory::Owner,
        BytecodeStructuralStorage::UniqueStructural,
    )
}

pub(in crate::codegen) fn structural_view_representation(
    chunk: &Chunk,
    ty: &SsaType,
) -> Option<BytecodeStructuralRepresentationId> {
    structural_representation(
        chunk,
        ty,
        BytecodeStructuralValueCategory::View,
        BytecodeStructuralStorage::BorrowedView,
    )
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
    field: u64,
) -> Result<u64> {
    let reference = StructuralDestinationFieldRef { destination, field };
    chunk.intern_structural_destination_field(reference)
}

pub(in crate::codegen) fn intern_aggregate_field(
    chunk: &mut Chunk,
    representation: lkjscript_ir::StructuralRepresentationId,
    field: u64,
    result: &SsaType,
    result_representation: Option<BytecodeStructuralRepresentationId>,
) -> Result<u64> {
    intern_aggregate_field_for_representation(
        chunk,
        BytecodeStructuralRepresentationId::new(representation.raw()),
        None,
        field,
        result,
        result_representation,
    )
}

pub(in crate::codegen) fn intern_aggregate_field_for_representation(
    chunk: &mut Chunk,
    representation: BytecodeStructuralRepresentationId,
    requested_variant: Option<BytecodeVariantId>,
    field: u64,
    result: &SsaType,
    result_representation: Option<BytecodeStructuralRepresentationId>,
) -> Result<u64> {
    let metadata = chunk
        .structural_representations
        .get(representation.index())
        .ok_or_else(|| Error::msg("aggregate field representation is missing"))?;
    let layout = chunk
        .structural_layouts
        .get(metadata.layout.index())
        .ok_or_else(|| Error::msg("aggregate field layout is missing"))?;
    let active_variant = match (&layout.kind, requested_variant) {
        (BytecodeStructuralLayoutKind::Enum { variants, .. }, Some(requested))
            if variants.iter().any(|variant| variant.variant == requested) =>
        {
            Some(requested)
        }
        (BytecodeStructuralLayoutKind::Enum { variants, .. }, None) if variants.len() == 1 => {
            Some(variants[0].variant)
        }
        (BytecodeStructuralLayoutKind::Enum { .. }, _) => {
            return Err(Error::msg(
                "aggregate enum field borrow requires exact active payload metadata",
            ))
        }
        (
            BytecodeStructuralLayoutKind::String
            | BytecodeStructuralLayoutKind::Path
            | BytecodeStructuralLayoutKind::Product { .. },
            None,
        ) => None,
        _ => {
            return Err(Error::msg(
                "aggregate product field unexpectedly names an enum variant",
            ))
        }
    };
    let reference = StructuralAggregateFieldRef {
        representation,
        active_variant,
        field,
        result: structural_field_from_chunk(chunk, result)?,
        result_representation,
    };
    chunk.intern_structural_aggregate_field(reference)
}

pub(in crate::codegen) fn intern_payload(
    chunk: &mut Chunk,
    representation: BytecodeStructuralRepresentationId,
    variant: lkjscript_ir::VariantId,
    result: &SsaType,
    result_representation: Option<BytecodeStructuralRepresentationId>,
) -> Result<u64> {
    let reference = StructuralPayloadRef {
        representation,
        variant: BytecodeVariantId::new(variant.bytes()),
        result: structural_field_from_chunk(chunk, result)?,
        result_representation,
    };
    chunk.intern_structural_payload(reference)
}

include!("metadata.rs");
