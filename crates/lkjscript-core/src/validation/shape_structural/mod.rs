use crate::{
    Chunk, Error, Result, StructuralDestinationMetadata, StructuralFieldMetadata,
    StructuralFieldRoute, StructuralLayoutKind, StructuralRepresentationMetadata,
    StructuralValueCategory, ValidationLimits, MAX_STRUCTURAL_DESTINATIONS, MAX_STRUCTURAL_LAYOUTS,
    MAX_STRUCTURAL_LAYOUT_FIELDS, MAX_STRUCTURAL_OPERATION_REFS, MAX_STRUCTURAL_REPRESENTATIONS,
    MAX_STRUCTURAL_TYPES,
};

pub(super) fn validate(chunk: &Chunk, limits: &ValidationLimits) -> Result<usize> {
    validate_table_limits(chunk, limits)?;
    let mut bytes = validate_witness_groups(chunk)?;
    bytes = add(
        bytes,
        validate_witnesses(chunk)?,
        "structural metadata bytes",
    )?;
    bytes = add(
        bytes,
        validate_layouts_and_types(chunk)?,
        "structural metadata bytes",
    )?;
    for (index, representation) in chunk.structural_representations.iter().enumerate() {
        if representation.id.index() != index {
            return Err(Error::msg(
                "bytecode structural RepresentationIds are not dense",
            ));
        }
        validate_representation(chunk, representation)?;
        bytes = add(bytes, 8, "structural metadata byte size")?;
    }
    for (index, destination) in chunk.structural_destinations.iter().enumerate() {
        if destination.id.index() != index {
            return Err(Error::msg(
                "bytecode structural DestinationIds are not dense",
            ));
        }
        validate_destination(chunk, destination)?;
        bytes = add(
            bytes,
            9_usize.saturating_add(destination.fields.len().saturating_mul(59)),
            "structural metadata byte size",
        )?;
    }
    validate_operation_references(chunk, bytes)
}

include!("tables.rs");
include!("witness_groups.rs");
include!("witnesses.rs");
include!("layouts.rs");
include!("references.rs");
include!("metadata.rs");
